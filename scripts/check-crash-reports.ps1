#Requires -Version 7
# Crash reports check: proves that WebView2 runs with custom crash reporting, that its dumps are
# swept, and that the app's own crashes skip Windows Error Reporting. It crashes nothing and needs
# no focus. Five probes:
#  - WebView2's browser runs with --edge-webview-disable-crash-reporting=1 on its command line, the
#    sign of custom crash reporting, so Crashpad uploads no dump;
#  - a dump planted in WebView2's report folder is gone by the time the app is ready;
#  - the app watches WebView2 for failed processes, whose dumps it sweeps;
#  - a dump planted while the app runs is gone once it has exited;
#  - the app's error mode has SEM_NOGPFAULTERRORBOX (0x2).
# It reads the app's debug echo (src-tauri/src/echo.rs) and the command lines of its child processes.
#
# Build first with `npm run tauri build -- --debug --no-bundle`. The app window opens briefly.
#
# Exit codes: 0 pass, 1 the positive control saw too little, 2 inconclusive, 3 a protection is
# missing. -PositiveControl starts the app with them off (KEYTRIAGE_CRASH_REPORTS, debug builds
# only) and exits 3 only if every probe caught it.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\debug\keytriage.exe'),
    [switch]$PositiveControl,
    [int]$StartTimeoutMs = 60000,
    [int]$StepTimeoutMs = 10000
)
$ErrorActionPreference = 'Stop'
$Hosting = 'windowed'

. (Join-Path $PSScriptRoot 'app-harness.ps1')

# Tauri gives WebView2 the app's local data folder, which is named after the identifier.
$identifier = (Get-Content (Join-Path $PSScriptRoot '..\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json).identifier
$crashpad = Join-Path $env:LOCALAPPDATA "$identifier\EBWebView\Crashpad"
$atStart = Join-Path $crashpad "reports\keytriage-check-$PID-start.dmp"
$attachment = Join-Path $crashpad "attachments\keytriage-check-$PID"
$atExit = Join-Path $crashpad "reports\keytriage-check-$PID-exit.dmp"

function Write-Dump([string]$path) {
    New-Item -ItemType Directory -Force (Split-Path $path) | Out-Null
    [IO.File]::WriteAllBytes($path, [Text.Encoding]::ASCII.GetBytes('MDMP synthetic, no key events'))
}

# Every keytriage sweeps the same folder and shares one WebView2 browser, so another one would
# answer for the app under test.
function Assert-Alone([int]$except) {
    if (@(Get-Process keytriage -ErrorAction SilentlyContinue | Where-Object Id -ne $except).Count -gt 0) {
        Stop-Inconclusive 'another keytriage is running'
    }
}

$app = $null
try {
    Assert-Alone 0
    Write-Dump $atStart
    New-Item -ItemType Directory -Force $attachment | Out-Null
    Set-Content (Join-Path $attachment 'note.txt') 'synthetic'

    $control = if ($PositiveControl) { 'KEYTRIAGE_CRASH_REPORTS' } else { $null }
    $app = [AppHarness.AppUnderTest]::Start((Resolve-Path $Exe), $false, $control)
    if (-not $app.WaitReady($StartTimeoutMs)) { Stop-Inconclusive 'the app did not print its ready line; is this a debug build?' }
    $caught = [System.Collections.Generic.List[string]]::new()

    # The startup sweep runs in setup, before the ready line, so no wait is needed.
    if ((Test-Path $atStart) -or (Test-Path $attachment)) { $caught.Add('a dump left in the report folder was still there after startup') }

    [void](Find-AppWindow $StartTimeoutMs)
    # The browser process is the WebView2 child without a --type switch.
    $tree = $W::ProcessTree([uint32]$app.Proc.Id)
    $browser = @(Get-CimInstance Win32_Process | Where-Object {
            $tree -contains $_.ProcessId -and $_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -notmatch ' --type='
        })
    if ($browser.Count -ne 1) { Stop-Inconclusive "found $($browser.Count) WebView2 browser processes of its own" }
    if ($browser[0].CommandLine -notmatch '--edge-webview-disable-crash-reporting=1') {
        $caught.Add('WebView2 runs without custom crash reporting, so Crashpad would upload its dumps')
    }

    if (-not $app.WaitFor({ $app.CrashWatch }, 3000)) { $caught.Add('the app does not watch WebView2 for failed processes') }

    if (-not $app.WaitFor({ $app.ErrorMode -ge 0 }, $StepTimeoutMs)) { Stop-Inconclusive 'the app reported no error mode' }
    if (($app.ErrorMode -band 0x2) -eq 0) {
        $caught.Add(('the app''s error mode 0x{0:x} lets its crashes reach Windows Error Reporting' -f $app.ErrorMode))
    }

    Assert-Alone $app.Proc.Id
    Write-Dump $atExit
    [void]$app.Proc.CloseMainWindow()
    if (-not $app.Proc.WaitForExit($StepTimeoutMs)) { Stop-Inconclusive 'the app did not exit when its window closed' }
    if (Test-Path $atExit) { $caught.Add('a dump written while the app ran was still there after it exited') }

    if ($PositiveControl) {
        $caught | ForEach-Object { Write-Host "caught: $_" }
        if ($caught.Count -lt 5) { Stop-Fail "with crash protections off, only $($caught.Count) of 5 probes caught it" }
        Write-Host 'FAIL (positive control): every probe caught its protection missing'
        exit 3
    }
    if ($caught.Count -gt 0) {
        $caught | ForEach-Object { Write-Host "FAIL: $_" }
        exit 3
    }
    Write-Host 'PASS: custom crash reporting is on, dumps are swept at startup, on failure and at exit, and the app skips Windows Error Reporting'
    exit 0
}
catch {
    Stop-Inconclusive "the harness failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    if ($app) { try { $app.Dispose() } catch {} }
    Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $atStart, $atExit, $attachment
}
