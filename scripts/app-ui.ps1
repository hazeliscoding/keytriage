# For the checks that start the release app. Dot-source it after app-harness.ps1: it loads UI
# Automation and defines the steps that start the app, find its rendered Start screen and read a
# startup dialog. They read the caller's $product, $apps and timeouts.

try { Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes }
catch { Stop-Inconclusive "UI Automation did not load: $_" }

$AE = [System.Windows.Automation.AutomationElement]
$TreeScope = [System.Windows.Automation.TreeScope]
function New-Condition([System.Windows.Automation.AutomationProperty]$property, $value, [switch]$IgnoreCase) {
    $flags = if ($IgnoreCase) { 'IgnoreCase' } else { 'None' }
    [System.Windows.Automation.PropertyCondition]::new($property, $value, [System.Windows.Automation.PropertyConditionFlags]$flags)
}
function Join-Conditions($a, $b) { [System.Windows.Automation.AndCondition]::new($a, $b) }
$IsButton = New-Condition ($AE::ControlTypeProperty) ([System.Windows.Automation.ControlType]::Button)
# pwsh's UI Automation client has no Win32 proxies, so a message box's controls show as panes with
# no patterns. Their window classes still tell them apart.
$IsWin32Button = New-Condition ($AE::ClassNameProperty) 'Button'
$IsWin32Text = New-Condition ($AE::ClassNameProperty) 'Static'

function Start-App([string]$exe, [hashtable]$set) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($exe)
    $psi.UseShellExecute = $false
    # WebView2 reads these, and a runner's own could move the data folder or pick another runtime.
    foreach ($name in @($psi.Environment.Keys)) {
        if ($name -like 'WEBVIEW2_*' -or $name -like 'COREWEBVIEW2_*') { [void]$psi.Environment.Remove($name) }
    }
    foreach ($name in $set.Keys) { $psi.Environment[$name] = $set[$name] }
    $proc = [System.Diagnostics.Process]::Start($psi)
    $apps.Add($proc)
    return $proc
}

# UI Automation reaches into WebView2's own process, and Chromium builds its accessibility tree only
# once a client asks, so the first searches can come back empty. Chromium names a button by its
# text after CSS, which can uppercase it.
function Find-Button([IntPtr]$hwnd, [string]$name, [int]$timeoutMs) {
    $condition = Join-Conditions $IsButton (New-Condition ($AE::NameProperty) $name -IgnoreCase)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        try {
            $found = $AE::FromHandle($hwnd).FindFirst($TreeScope::Descendants, $condition)
            if ($found) { return $found }
        }
        # The tree can change under a search, and the next one tells.
        catch [System.Windows.Automation.ElementNotAvailableException] {}
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

# A dialog can be found before its controls exist, so it counts once it holds a button.
function Find-Dialog([System.Diagnostics.Process]$proc, [int]$timeoutMs) {
    $condition = Join-Conditions (New-Condition ($AE::ProcessIdProperty) $proc.Id) (New-Condition ($AE::ClassNameProperty) '#32770')
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        try {
            $found = $AE::RootElement.FindFirst($TreeScope::Children, $condition)
            if ($found -and $found.FindFirst($TreeScope::Descendants, $IsWin32Button)) { return $found }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] {}
        if ($proc.HasExited) { return $null }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

# Process.CloseMainWindow can pick another window the process owns, such as the input indicator's.
function Close-Window([System.Windows.Automation.AutomationElement]$window) {
    $window.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
}

function Get-DialogText($dialog) {
    @($dialog.FindAll($TreeScope::Descendants, $IsWin32Text) | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' '
}

# The Begin test button is on the Start screen even without a keyboard, so finding it proves
# WebView2 started with the app's environment and the bundled page rendered.
function Wait-StartScreen([System.Diagnostics.Process]$app, [string]$what) {
    $hwnd = [IntPtr]::Zero
    $deadline = [DateTime]::UtcNow.AddMilliseconds($WindowTimeoutMs)
    while ($hwnd -eq [IntPtr]::Zero -and -not $app.HasExited -and [DateTime]::UtcNow -lt $deadline) {
        $hwnd = $W::FindTopLevel($app.Id, $product)
        if ($hwnd -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        if ($app.HasExited) { Stop-Fail "$what exited with $($app.ExitCode) before it showed a window" }
        Stop-Fail "$what showed no window titled $product"
    }
    if ($W::Class($hwnd) -eq '#32770') { Stop-Fail "$what couldn't start: $(Get-DialogText ($AE::FromHandle($hwnd)))" }
    $button = Find-Button $hwnd 'Begin test' $RenderTimeoutMs
    if (-not $button) { Stop-Fail "the page never rendered its Begin test button within $($RenderTimeoutMs / 1000) s" }
    Write-Host "$what rendered its Start screen (Begin test enabled: $($button.Current.IsEnabled))"
    return $hwnd
}

function Close-App([System.Diagnostics.Process]$app, [IntPtr]$hwnd) {
    Close-Window ($AE::FromHandle($hwnd))
    if (-not $app.WaitForExit($ExitTimeoutMs)) { Stop-Fail "the app did not exit within $($ExitTimeoutMs / 1000) s of its window closing" }
    Write-Host "the app exited with $($app.ExitCode)"
}

# A start that can't go on shows one message box, titled with the product and holding only OK, and
# exits with 1 once it closes. Returns the box's text.
function Read-ExitDialog([System.Diagnostics.Process]$app, [string]$when, [string]$like) {
    $dialog = Find-Dialog $app $DialogTimeoutMs
    if (-not $dialog) {
        if ($app.HasExited) { Stop-Fail "$when the app exited with $($app.ExitCode) and showed no dialog" }
        Stop-Fail "$when the app showed no dialog within $($DialogTimeoutMs / 1000) s"
    }
    $text = Get-DialogText $dialog
    if ($dialog.Current.Name -ne $product -or $text -notlike $like) {
        Stop-Fail "$when the dialog is titled '$($dialog.Current.Name)' and reads '$text'"
    }
    # The box has only OK, so closing it returns OK. Its button offers no Invoke to pwsh's client.
    $buttons = @($dialog.FindAll($TreeScope::Descendants, $IsWin32Button))
    if ($buttons.Count -ne 1) { Stop-Fail "$when the dialog has $($buttons.Count) buttons, not one" }
    Close-Window $dialog
    if (-not $app.WaitForExit($ExitTimeoutMs)) { Stop-Fail "$when the app did not exit after its dialog closed" }
    if ($app.ExitCode -ne 1) { Stop-Fail "$when the app exited with $($app.ExitCode) after its dialog, not 1" }
    return $text
}
