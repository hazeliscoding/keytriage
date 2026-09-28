# Shared by the app checks in scripts/. Dot-source it first: it loads app-harness.cs and defines
# the exit helpers and the foreground and focus steps. Test-Focus reads the caller's $Hosting and
# $app.

function Stop-Inconclusive([string]$why) { Write-Host "INCONCLUSIVE: $why"; exit 2 }
function Stop-Fail([string]$why) { Write-Host "FAIL: $why"; exit 1 }
# Exit 3 is kept for the one failure a check exists to catch, so its positive control can only
# pass through that assertion.
function Stop-Caught([string]$why) { Write-Host "FAIL: $why"; exit 3 }

try {
    Add-Type -Path (Join-Path $PSScriptRoot 'app-harness.cs') -ReferencedAssemblies @(
        'System.Windows.Forms', 'System.Windows.Forms.Primitives', 'System.ComponentModel.Primitives',
        'System.Drawing.Primitives', 'System.Collections', 'System.Threading', 'System.Threading.Thread',
        'System.Diagnostics.Process', 'System.Runtime.InteropServices', 'System.Text.RegularExpressions',
        'System.ComponentModel'
    )
}
catch { Stop-Inconclusive "the harness did not load: $_" }
$W = [AppHarness.Win]

function Enter-Foreground([IntPtr]$hwnd, [string]$name) {
    $how = $W::Activate($hwnd, 2000)
    if (-not $how) { Stop-Inconclusive "could not bring $name to the foreground" }
    Write-Host "$name is in the foreground ($how)"
}

function Assert-Foreground([IntPtr]$hwnd, [string]$phase) {
    $fg = $W::GetForegroundWindow()
    if ($fg -ne $hwnd) { Stop-Inconclusive "the foreground changed during $phase (now $($W::Describe($fg)))" }
}

function Find-AppWindow([int]$timeoutMs) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        $hwnd = $W::FindTopLevel($app.Proc.Id, 'keytriage')
        if ($hwnd -ne [IntPtr]::Zero) { return $hwnd }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    Stop-Inconclusive 'no visible window titled keytriage in the app process'
}

# Windowed hosting, the default, puts keyboard focus in WebView2's own process. Visual hosting
# keeps it in the app's process.
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

# The app in the foreground, with keyboard focus inside the WebView2 content.
function Enter-App([IntPtr]$hwnd) {
    Enter-Foreground $hwnd 'the app'
    if (-not (Wait-Focus $hwnd)) {
        if (-not $W::ClickClientCenter($hwnd)) { Stop-Inconclusive 'could not click into the app window' }
        if (-not (Wait-Focus $hwnd)) { Stop-Inconclusive "focus is not where $Hosting hosting puts it ($($W::FocusInfo($hwnd)))" }
    }
    Write-Host "focus: $($W::FocusInfo($hwnd))"
}
