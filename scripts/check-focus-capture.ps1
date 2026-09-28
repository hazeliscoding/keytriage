#Requires -Version 7
# Focus check: proves that the app receives Raw Input while its window is in the foreground, and
# none while another process's window is, when capture pauses and gives up its registration. The
# same holds for the Pause button with the app still in front. It injects F13, F14 and F15 by scan
# code, which nothing in the app or the browser binds, and reads the app's debug echo
# (src-tauri/src/echo.rs), which starts a test once the page has loaded. F17 makes the page pause
# the test and continue it 3 s later, as the Pause and Continue buttons do.
#
# Build first with `npm run tauri build -- --debug --no-bundle`. The run takes the foreground and
# may click the middle of the app window. Don't type while it runs.
#
# -Hosting picks WebView2's hosting mode. Windowed, the default, puts keyboard focus in WebView2's
# own process, which is the case this check exists for. Visual (window-to-visual) keeps focus in
# the app's process. Users can force either mode, so run both.
#
# Exit codes: 0 pass, 1 capture is missing or registered wrongly, 2 inconclusive (a phase could
# not be set up, so the run proves nothing either way), 3 the app captured in the background or
# during a user pause, or didn't pause and give up its registration then. -PositiveControl
# Background keeps the app in front during the background phase and must exit 3 through the
# background keys; -PositiveControl Registration keeps a registration while paused
# (KEYTRIAGE_KEEP_REGISTRATION, debug builds only) and must exit 3 through the paused registration;
# -PositiveControl UserPause keeps capture running through a user pause
# (KEYTRIAGE_USER_PAUSE_CAPTURES, debug builds only) and must exit 3 through the paused
# registration.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\debug\keytriage.exe'),
    [ValidateSet('windowed', 'visual')][string]$Hosting = 'windowed',
    [ValidateSet('', 'Background', 'Registration', 'UserPause')][string]$PositiveControl = '',
    [int]$Taps = 5,
    [int]$StepTimeoutMs = 5000,
    [int]$StartTimeoutMs = 60000
)
$ErrorActionPreference = 'Stop'

$F13 = 0x64; $F14 = 0x65; $F15 = 0x66; $F17 = 0x68

. (Join-Path $PSScriptRoot 'app-harness.ps1')

# Registration is per process and the last call wins, so the process must hold exactly one: the
# keyboard, aimed at the app window, with no sink flag (RIDEV_INPUTSINK 0x100, RIDEV_EXINPUTSINK
# 0x1000).
function Assert-Registration([IntPtr]$hwnd, [string]$phase) {
    $regs = $app.Registrations
    if ($regs.Count -ne 1) { Stop-Fail "in $phase the process holds $($regs.Count) Raw Input registrations, not 1" }
    $reg = $regs[0]
    if ($reg.Page -ne 1 -or $reg.Usage -ne 6) { Stop-Fail ('in {0} the registration is page 0x{1:x} usage 0x{2:x}, not the keyboard' -f $phase, $reg.Page, $reg.Usage) }
    if ($reg.Flags -band 0x1100) { Stop-Fail ('in {0} the registration flags 0x{1:x} include an input sink' -f $phase, $reg.Flags) }
    if ($reg.Target -ne $hwnd.ToInt64()) { Stop-Fail "in $phase the registration does not target the app window" }
    Write-Host ('{0}: keyboard registration, flags 0x{1:x}, targets the app window' -f $phase, $reg.Flags)
}

$app = $null
$probe = $null
try {
    $switch = switch ($PositiveControl) {
        'Registration' { 'KEYTRIAGE_KEEP_REGISTRATION' }
        'UserPause' { 'KEYTRIAGE_USER_PAUSE_CAPTURES' }
        default { $null }
    }
    $app = [AppHarness.AppUnderTest]::Start((Resolve-Path $Exe), $Hosting -eq 'visual', $switch)
    Wait-TestStarted $StartTimeoutMs $StepTimeoutMs
    $hwnd = Find-AppWindow $StartTimeoutMs

    # Phase 1: the app is in the foreground with focus inside the WebView2 content. Capture
    # registers, if it didn't at the start of the test, and events arrive.
    Enter-App $hwnd
    if (-not $app.WaitFor({ $app.Registrations.Count -ge 1 }, $StepTimeoutMs)) { Stop-Fail 'capture did not register with the app in the foreground' }
    Assert-Registration $hwnd 'phase 1'
    Assert-Foreground $hwnd 'phase 1'
    if ($W::Tap($F13, $Taps) -ne 2 * $Taps) { Stop-Inconclusive 'SendInput was blocked (UIPI or a secure desktop)' }
    $got = $app.WaitDowns($F13, $Taps, $StepTimeoutMs)
    Assert-Foreground $hwnd 'phase 1'
    if (-not $got) { Stop-Fail "the app in the foreground received $($app.Downs($F13)) of $Taps key downs" }

    # Phase 2: another process's window is in the foreground. Capture pauses and unregisters. The
    # probe sees every key; the app must see none.
    $pauses = $app.Pauses
    $resumes = $app.Resumes
    $snapshots = $app.Snapshots
    if ($PositiveControl -eq 'Background') {
        [void]$W::Tap($F14, $Taps)
        [void]$app.WaitDowns($F14, $Taps, $StepTimeoutMs)
        Assert-Foreground $hwnd 'phase 2'
    }
    else {
        $probe = [AppHarness.ProbeForm]::Start('keytriage focus probe', $F14)
        Enter-Foreground $probe.Hwnd 'the probe window'
        [void]$W::Tap($F14, $Taps)
        if (-not $probe.WaitCount($Taps, $StepTimeoutMs)) { Stop-Inconclusive "the probe window received $($probe.Count) of $Taps key downs" }
        Assert-Foreground $probe.Hwnd 'phase 2'
        if (-not $app.WaitFor({ $app.Pauses -gt $pauses }, $StepTimeoutMs)) { Stop-Caught 'capture did not pause when the app lost the foreground' }
        # The same count reads 1 after every resume, so it can see a registration when there is one.
        if ($app.PausedRegistrations -eq -2) { Stop-Fail 'the app could not read its registrations while paused' }
        if ($app.PausedRegistrations -ne 0) { Stop-Caught "capture kept $($app.PausedRegistrations) Raw Input registrations while paused" }
    }

    # Phase 3: back to the app. The harness owns the foreground now, so a plain activation works.
    # WM_INPUT is queued in order, so once these arrive, any phase 2 event would already have
    # been counted.
    Enter-Foreground $hwnd 'the app'
    if ($PositiveControl -ne 'Background') {
        if (-not $app.WaitFor({ $app.Resumes -gt $resumes -and $app.Snapshots -gt $snapshots }, $StepTimeoutMs)) {
            Stop-Fail 'capture did not resume when the app came back'
        }
        Assert-Registration $hwnd 'phase 3'
    }
    [void]$W::Tap($F15, $Taps)
    $got = $app.WaitDowns($F15, $Taps, $StepTimeoutMs)
    Assert-Foreground $hwnd 'phase 3'
    if (-not $got) { Stop-Fail "events did not resume ($($app.Downs($F15)) of $Taps)" }
    if ($app.Downs($F14) -ne 0) { Stop-Caught "the app received $($app.Downs($F14)) key downs while in the background" }

    # Phase 4: the user pauses the test with the app still in front. Capture stops and unregisters,
    # so the app reads none of the keys that reach its page, until Continue registers again.
    $pauses = $app.Pauses
    $resumes = $app.Resumes
    $snapshots = $app.Snapshots
    $others = $app.OtherDowns
    Assert-Foreground $hwnd 'phase 4'
    [void]$W::Tap($F17, 1)
    if (-not $app.WaitFor({ $app.OtherDowns -gt $others }, $StepTimeoutMs)) { Stop-Fail 'the app in the foreground did not receive F17' }
    if (-not $app.WaitFor({ $app.Pauses -gt $pauses }, $StepTimeoutMs)) { Stop-Fail 'the test did not pause when the page asked' }
    if ($app.PausedRegistrations -eq -2) { Stop-Fail 'the app could not read its registrations while paused' }
    if ($app.PausedRegistrations -ne 0) { Stop-Caught "capture kept $($app.PausedRegistrations) Raw Input registrations through a user pause" }
    Assert-Foreground $hwnd 'phase 4'
    [void]$W::Tap($F14, $Taps)
    if (-not $app.WaitFor({ $app.Resumes -gt $resumes -and $app.Snapshots -gt $snapshots }, $StepTimeoutMs)) {
        Stop-Fail 'capture did not resume when the page continued the test'
    }
    Assert-Registration $hwnd 'phase 4'
    # As in phase 3, once these arrive any key read during the pause would already be counted.
    [void]$W::Tap($F15, $Taps)
    $got = $app.WaitDowns($F15, 2 * $Taps, $StepTimeoutMs)
    Assert-Foreground $hwnd 'phase 4'
    if (-not $got) { Stop-Fail "events did not resume after the user pause ($($app.Downs($F15) - $Taps) of $Taps)" }
    if ($app.Downs($F14) -ne 0) { Stop-Caught "the app received $($app.Downs($F14)) key downs during a user pause" }

    # F17 is the one key down the echo hides.
    if ($app.OtherDowns -ne 1) { Stop-Inconclusive "$($app.OtherDowns - 1) key downs arrived that the harness did not send" }
    # A test that starts in the background begins paused, so every resume follows a pause.
    if ($app.Resumes -gt $app.Pauses) { Stop-Fail "the app resumed $($app.Resumes) times after only $($app.Pauses) pauses" }

    Write-Host "injected keys arrived with device handle $($app.MarkerDevices)"
    Write-Host ('PASS ({1} hosting): foreground {0}/{0}, paused and unregistered, background 0/{0}, resumed {0}/{0}, user pause 0/{0}, continued {0}/{0}' -f $Taps, $Hosting)
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
