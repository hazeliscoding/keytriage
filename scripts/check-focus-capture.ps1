#Requires -Version 7
# Focus check: proves that the app receives Raw Input while its window is in the foreground, and
# none while another process's window is. It injects F13, F14 and F15 by scan code, which nothing
# in the app or the browser binds, and reads the app's debug echo (src-tauri/src/echo.rs).
#
# Build first with `npm run tauri build -- --debug --no-bundle`. The run takes the foreground and
# may click the middle of the app window. Don't type while it runs.
#
# -Hosting picks WebView2's hosting mode. Windowed, the default, puts keyboard focus in WebView2's
# own process, which is the case this check exists for. Visual (window-to-visual) keeps focus in
# the app's process. Users can force either mode, so run both.
#
# Exit codes: 0 pass, 1 capture is missing or registered wrongly, 2 inconclusive (a phase could
# not be set up, so the run proves nothing either way), 3 the app captured in the background.
# -PositiveControl keeps the app in front during the background phase, so a working check exits 3.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\debug\keytriage.exe'),
    [ValidateSet('windowed', 'visual')][string]$Hosting = 'windowed',
    [switch]$PositiveControl,
    [int]$Taps = 5,
    [int]$StepTimeoutMs = 5000,
    [int]$StartTimeoutMs = 60000
)
$ErrorActionPreference = 'Stop'

$F13 = 0x64; $F14 = 0x65; $F15 = 0x66

function Stop-Inconclusive([string]$why) { Write-Host "INCONCLUSIVE: $why"; exit 2 }
function Stop-Fail([string]$why) { Write-Host "FAIL: $why"; exit 1 }
function Stop-Captured([string]$why) { Write-Host "FAIL: $why"; exit 3 }

function Enter-Foreground([IntPtr]$hwnd, [string]$name) {
    $how = $W::Activate($hwnd, 2000)
    if (-not $how) { Stop-Inconclusive "could not bring $name to the foreground" }
    Write-Host "$name is in the foreground ($how)"
}

function Assert-Foreground([IntPtr]$hwnd, [string]$phase) {
    $fg = $W::GetForegroundWindow()
    if ($fg -ne $hwnd) { Stop-Inconclusive "the foreground changed during $phase (now class $($W::Class($fg)))" }
}

function Test-Focus([IntPtr]$hwnd) {
    $owner = $W::FocusProcess($hwnd)
    if ($Hosting -eq 'windowed') { return $owner -ne 0 -and $owner -ne $app.Proc.Id }
    return $owner -ne 0
}

# WebView2 moves focus into its content asynchronously, across processes.
function Wait-Focus([IntPtr]$hwnd) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds(2000)
    do {
        if (Test-Focus $hwnd) { return $true }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    return $false
}

$app = $null
$probe = $null
try {
    Add-Type -Path (Join-Path $PSScriptRoot 'check-focus-capture.cs') -ReferencedAssemblies @(
        'System.Windows.Forms', 'System.Windows.Forms.Primitives', 'System.ComponentModel.Primitives',
        'System.Drawing.Primitives', 'System.Collections', 'System.Threading', 'System.Threading.Thread',
        'System.Diagnostics.Process', 'System.Runtime.InteropServices', 'System.Text.RegularExpressions',
        'System.ComponentModel'
    )
    $W = [FocusCheck.Win]

    $app = [FocusCheck.AppUnderTest]::Start((Resolve-Path $Exe), $Hosting -eq 'visual')
    if (-not $app.WaitReady($StartTimeoutMs)) { Stop-Inconclusive 'the app did not print its ready line; is this a debug build?' }

    $hwnd = [IntPtr]::Zero
    $deadline = [DateTime]::UtcNow.AddMilliseconds($StartTimeoutMs)
    while ($hwnd -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline) {
        $hwnd = $W::FindTopLevel($app.Proc.Id, 'keytriage')
        if ($hwnd -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 100 }
    }
    if ($hwnd -eq [IntPtr]::Zero) { Stop-Inconclusive 'no visible window titled keytriage in the app process' }

    # Registration is per process and the last call wins, so the process must hold exactly one:
    # the keyboard, aimed at the app window, with no sink flag (RIDEV_INPUTSINK 0x100,
    # RIDEV_EXINPUTSINK 0x1000).
    $regs = $app.Registrations
    if ($regs.Count -ne 1) { Stop-Fail "the process holds $($regs.Count) Raw Input registrations, not 1" }
    $reg = $regs[0]
    if ($reg.Page -ne 1 -or $reg.Usage -ne 6) { Stop-Fail ('the registration is page 0x{0:x} usage 0x{1:x}, not the keyboard' -f $reg.Page, $reg.Usage) }
    if ($reg.Flags -band 0x1100) { Stop-Fail ('the registration flags 0x{0:x} include an input sink' -f $reg.Flags) }
    if ($reg.Target -ne $hwnd.ToInt64()) { Stop-Fail 'the registration does not target the app window' }
    Write-Host ('registration: keyboard, flags 0x{0:x}, targets the app window' -f $reg.Flags)

    # Phase 1: the app is in the foreground with focus inside the WebView2 content. Events arrive.
    Enter-Foreground $hwnd 'the app'
    if (-not (Wait-Focus $hwnd)) {
        if (-not $W::ClickClientCenter($hwnd)) { Stop-Inconclusive 'could not click into the app window' }
        if (-not (Wait-Focus $hwnd)) { Stop-Inconclusive "focus is not where $Hosting hosting puts it ($($W::FocusInfo($hwnd)))" }
    }
    Write-Host "focus: $($W::FocusInfo($hwnd))"
    Assert-Foreground $hwnd 'phase 1'
    if ($W::Tap($F13, $Taps) -ne 2 * $Taps) { Stop-Inconclusive 'SendInput was blocked (UIPI or a secure desktop)' }
    $got = $app.WaitDowns($F13, $Taps, $StepTimeoutMs)
    Assert-Foreground $hwnd 'phase 1'
    if (-not $got) { Stop-Fail "the app in the foreground received $($app.Downs($F13)) of $Taps key downs" }

    # Phase 2: another process's window is in the foreground. The probe sees every key; the app
    # must see none.
    if ($PositiveControl) {
        [void]$W::Tap($F14, $Taps)
        [void]$app.WaitDowns($F14, $Taps, $StepTimeoutMs)
        Assert-Foreground $hwnd 'phase 2'
    }
    else {
        $probe = [FocusCheck.ProbeForm]::Start('keytriage focus probe', $F14)
        Enter-Foreground $probe.Hwnd 'the probe window'
        [void]$W::Tap($F14, $Taps)
        if (-not $probe.WaitCount($Taps, $StepTimeoutMs)) { Stop-Inconclusive "the probe window received $($probe.Count) of $Taps key downs" }
        Assert-Foreground $probe.Hwnd 'phase 2'
    }

    # Phase 3: back to the app. The harness owns the foreground now, so a plain activation works.
    # WM_INPUT is queued in order, so once these arrive, any phase 2 event would already have
    # been counted.
    Enter-Foreground $hwnd 'the app'
    [void]$W::Tap($F15, $Taps)
    $got = $app.WaitDowns($F15, $Taps, $StepTimeoutMs)
    Assert-Foreground $hwnd 'phase 3'
    if (-not $got) { Stop-Fail "events did not resume ($($app.Downs($F15)) of $Taps)" }

    if ($app.Downs($F14) -ne 0) { Stop-Captured "the app received $($app.Downs($F14)) key downs while in the background" }
    if ($app.OtherDowns -ne 0) { Stop-Inconclusive "$($app.OtherDowns) key downs arrived that the harness did not send" }

    Write-Host "injected keys arrived with device handle $($app.MarkerDevices)"
    Write-Host ('PASS ({1} hosting): foreground {0}/{0}, background 0/{0}, resumed {0}/{0}' -f $Taps, $Hosting)
    exit 0
}
catch {
    Stop-Inconclusive "the harness failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    if ($probe) { try { $probe.CloseFromAnyThread() } catch {} }
    if ($app) { try { $app.Dispose() } catch {} }
}
