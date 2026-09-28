#Requires -Version 7
# Browser keys check: proves that WebView2's own shortcuts and context menu do nothing in the app,
# so a key test can press every key. It reads the app's debug echo (src-tauri/src/echo.rs): the
# settings WebView2 reports, and a line per page load, which a reload adds. It taps F5 and Ctrl+R,
# then right-clicks the page and looks for a menu window from the app's own WebView2 processes.
# Ctrl+P is covered by the same setting and left out, because a print dialog is hard to close.
#
# Build first with `npm run tauri build -- --debug --no-bundle`. The run takes the foreground and
# clicks the middle of the app window. Don't type while it runs.
#
# Exit codes: 0 pass, 1 the settings read back wrong or the positive control saw too little, 2
# inconclusive, 3 a browser key or the context menu acted. -PositiveControl starts the app with
# them left on, expects the settings to read back as on, and exits 3 only if every probe caught
# its action.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\debug\keytriage.exe'),
    [ValidateSet('windowed', 'visual')][string]$Hosting = 'windowed',
    [switch]$PositiveControl,
    [int]$SettleMs = 3000,
    [int]$StartTimeoutMs = 60000
)
$ErrorActionPreference = 'Stop'

$Esc = 0x01; $Ctrl = 0x1D; $F5 = 0x3F; $VkR = 0x52

. (Join-Path $PSScriptRoot 'app-harness.ps1')

$app = $null
try {
    $app = [AppHarness.AppUnderTest]::Start((Resolve-Path $Exe), $Hosting -eq 'visual', $(if ($PositiveControl) { 'KEYTRIAGE_BROWSER_KEYS' } else { $null }))
    if (-not $app.WaitReady($StartTimeoutMs)) { Stop-Inconclusive 'the app did not print its ready line; is this a debug build?' }
    $hwnd = Find-AppWindow $StartTimeoutMs
    if (-not $app.WaitFor({ $app.Settings -and $app.PageLoads -ge 1 }, $StartTimeoutMs)) {
        Stop-Inconclusive 'the app reported no WebView2 settings or no first page load'
    }
    Write-Host "settings: $($app.Settings)"
    $want = if ($PositiveControl) { 'browser-keys=1 context-menus=1' } else { 'browser-keys=0 context-menus=0' }
    if ($app.Settings -ne $want) { Stop-Fail "WebView2 reports $($app.Settings), not $want" }

    Enter-App $hwnd
    $R = $W::ScanFor($VkR, $hwnd)
    if ($R -eq 0) { Stop-Inconclusive 'the keyboard layout has no R key' }
    $caught = [System.Collections.Generic.List[string]]::new()

    # Another window can take the foreground mid-probe, as a terminal did in development. A probe
    # counts only if the app stayed in front for all of it, so it gets three tries. A probe returns
    # whether the browser acted, or 'unseen' when its input never reached the app.
    function Test-Probe([string]$name, [scriptblock]$probe) {
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            if ($W::GetForegroundWindow() -ne $hwnd) { Enter-App $hwnd }
            $result = & $probe
            $fg = $W::GetForegroundWindow()
            if ($fg -ne $hwnd) { Write-Host "the foreground moved to $($W::Describe($fg)) during $name, trying again"; continue }
            if ($result -is [string]) { Stop-Inconclusive "$name never reached the app (UIPI or a secure desktop)" }
            return [bool]$result
        }
        Stop-Inconclusive "the foreground kept moving during $name"
    }

    # A reload adds a page load. The app's own capture sees each key down, which proves it arrived.
    function Test-Reload([int]$downs, [scriptblock]$send) {
        $loads = $app.PageLoads
        $seen = $app.OtherDowns
        [void](& $send)
        if (-not $app.WaitFor({ $app.OtherDowns -ge $seen + $downs }, $SettleMs)) { return 'unseen' }
        return $app.WaitFor({ $app.PageLoads -gt $loads }, $SettleMs)
    }

    if (Test-Probe 'F5' { Test-Reload 1 { $W::Tap($F5, 1) } }) { $caught.Add('F5 reloaded the page') }
    if (Test-Probe 'Ctrl+R' { Test-Reload 2 { $W::Chord($Ctrl, $R) } }) { $caught.Add('Ctrl+R reloaded the page') }

    # The context menu is a top-level window of WebView2's browser process, which the app started.
    $tree = $W::ProcessTree([uint32]$app.Proc.Id)
    # WebView2 shares one browser process per data folder, so a keytriage that is already running
    # would own the menu, out of this check's sight.
    if (-not ($tree | Where-Object { (Get-Process -Id $_ -ErrorAction SilentlyContinue).ProcessName -eq 'msedgewebview2' })) {
        Stop-Inconclusive "the app's WebView2 browser process is not its own; is another keytriage running?"
    }
    $menu = Test-Probe 'the right-click' {
        $before = $W::VisibleTopLevel($tree, $hwnd)
        if (-not $W::ClickClientCenter($hwnd, $true)) { return 'unseen' }
        $opened = $app.WaitFor({ @($W::VisibleTopLevel($tree, $hwnd) | Where-Object { $before -notcontains $_ }).Count -gt 0 }, $SettleMs)
        if ($opened) { [void]$W::Tap($Esc, 1); [void]$app.WaitFor({ $W::GetForegroundWindow() -eq $hwnd }, 2000) }
        $opened
    }
    if ($menu) { $caught.Add('a right-click opened a context menu') }

    if ($PositiveControl) {
        $caught | ForEach-Object { Write-Host "caught: $_" }
        if ($caught.Count -lt 3) { Stop-Fail "with browser keys left on, only $($caught.Count) of 3 probes caught their action" }
        Write-Host 'FAIL (positive control): every probe caught its action'
        exit 3
    }
    if ($caught.Count -gt 0) {
        $caught | ForEach-Object { Write-Host "FAIL: $_" }
        exit 3
    }
    Write-Host "PASS ($Hosting hosting): F5 and Ctrl+R did not reload, and a right-click opened no menu"
    exit 0
}
catch {
    Stop-Inconclusive "the harness failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    if ($app) { try { $app.Dispose() } catch {} }
}
