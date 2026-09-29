#Requires -Version 7
# Installer check: installs the NSIS installer silently and checks what it put where, starts the
# installed app and finds its rendered Begin test button, checks the app's data folders and the
# missing-WebView2 dialog, then uninstalls silently and checks that the app is gone.
#
# It installs software, so it runs only on a CI runner (GITHUB_ACTIONS=true), or with -AllowLocal
# in a throwaway VM. It stops before installing when keytriage is or was installed, and afterwards
# removes what its run created, including the data a silent uninstall keeps.
#
# Exit codes: 0 pass, 1 fail, 2 inconclusive, 3 something left behind that the check exists to
# catch. Each positive control must exit 3: -PositiveControl Leftovers skips the uninstaller, and
# DataFile plants a file in the app's data folder before it is checked.
param(
    [Parameter(Mandatory)][string]$Installer,
    [ValidateSet('Leftovers', 'DataFile')][string]$PositiveControl,
    [switch]$AllowLocal,
    [int]$WindowTimeoutMs = 30000,
    [int]$RenderTimeoutMs = 60000,
    [int]$DialogTimeoutMs = 20000,
    [int]$ExitTimeoutMs = 10000,
    [int]$UninstallTimeoutMs = 60000
)
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'app-harness.ps1')

if ($env:GITHUB_ACTIONS -ne 'true' -and -not $AllowLocal) {
    Stop-Inconclusive 'this installs keytriage, so it runs only on a CI runner, or with -AllowLocal in a throwaway VM'
}
try { Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes }
catch { Stop-Inconclusive "UI Automation did not load: $_" }

$config = Get-Content (Join-Path $PSScriptRoot '..\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$product = $config.productName
$publisher = $config.bundle.publisher
# Empty names would point the cleanup at %LOCALAPPDATA% itself.
if (-not $product -or -not $publisher -or -not $config.identifier) { Stop-Inconclusive 'tauri.conf.json lacks productName, identifier or bundle.publisher' }
$file = Split-Path -Leaf $Installer
if ($file -notmatch "^$([regex]::Escape($product))_(\d+\.\d+\.\d+)_x64-setup\.exe$") {
    Stop-Inconclusive "$file is not named ${product}_X.Y.Z_x64-setup.exe"
}
$version = $Matches[1]
if (-not (Test-Path -LiteralPath $Installer -PathType Leaf)) { Stop-Inconclusive "no installer at $Installer" }
$Installer = (Resolve-Path -LiteralPath $Installer).Path

# Where Tauri's NSIS template puts a per-user install, and where the app keeps its data.
$installDir = Join-Path $env:LOCALAPPDATA $product
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$product"
$publisherKey = "HKCU:\Software\$publisher"
$productKey = "$publisherKey\$product"
$localData = Join-Path $env:LOCALAPPDATA $config.identifier
$roamingData = Join-Path $env:APPDATA $config.identifier
$exe = Join-Path $installDir "$product.exe"

$present = @($installDir, $uninstallKey, $productKey, $localData, $roamingData) | Where-Object { Test-Path -LiteralPath $_ }
if ($present) { Stop-Inconclusive "keytriage is or was installed here: $($present -join ', ')" }

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

$apps = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()

function Start-App([hashtable]$set) {
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

# WebView2 keeps its profile in EBWebView. Anything else the app writes there, or a roaming folder,
# is data the privacy contract doesn't account for.
function Get-DataProblems([string]$local, [string]$roaming) {
    if (Test-Path -LiteralPath $roaming) { "$roaming exists" }
    Get-ChildItem -LiteralPath $local -Force | Where-Object Name -ne 'EBWebView' | ForEach-Object { "$local holds $($_.Name)" }
}

function Get-Leftovers { @($installDir, $uninstallKey) | Where-Object { Test-Path -LiteralPath $_ } }

# Without _?= the uninstaller copies itself to %TEMP% and returns at once, so wait for its work.
function Wait-Uninstalled([int]$timeoutMs) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        $left = @(Get-Leftovers)
        if (-not $left) { return @() }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    return $left
}

function Invoke-Uninstaller {
    $un = Start-Process -FilePath (Join-Path $installDir 'uninstall.exe') -ArgumentList '/S' -Wait -PassThru
    Write-Host "the uninstaller returned $($un.ExitCode)"
}

$noRuntime = Join-Path ([IO.Path]::GetTempPath()) "keytriage-no-runtime-$PID"
try {
    $setup = Start-Process -FilePath $Installer -ArgumentList '/S', '/NS' -Wait -PassThru
    if ($setup.ExitCode -ne 0) { Stop-Fail "the installer exited with $($setup.ExitCode)" }

    $files = 'keytriage.exe', 'uninstall.exe', 'licenses\LICENSE.txt', 'licenses\THIRD-PARTY-NPM.txt', 'licenses\THIRD-PARTY-RUST.txt'
    $missing = $files | Where-Object {
        $path = Join-Path $installDir $_
        -not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-Item -LiteralPath $path).Length -eq 0
    }
    if ($missing) { Stop-Fail "the install folder lacks $($missing -join ', ')" }

    $entry = Get-ItemProperty -LiteralPath $uninstallKey -ErrorAction SilentlyContinue
    if (-not $entry) { Stop-Fail "no Uninstall entry at $uninstallKey" }
    $expected = [ordered]@{ DisplayName = $product; DisplayVersion = $version; Publisher = $publisher; URLInfoAbout = $config.bundle.homepage }
    foreach ($name in $expected.Keys) {
        if ($entry.$name -ne $expected[$name]) { Stop-Fail "the Uninstall entry's $name is '$($entry.$name)', not '$($expected[$name])'" }
    }
    if (-not (Test-Path -LiteralPath $productKey)) { Stop-Fail "no $productKey, where NSIS remembers the install folder" }
    $info = (Get-Item -LiteralPath $exe).VersionInfo
    if ($info.CompanyName -ne $publisher -or $info.ProductVersion -ne $version) {
        Stop-Fail "the exe's version info reads CompanyName '$($info.CompanyName)' and ProductVersion '$($info.ProductVersion)'"
    }
    Write-Host "installed $version into $installDir, with its three license files and an Uninstall entry by $publisher"

    # The Begin test button is on the Start screen even without a keyboard, so finding it proves
    # WebView2 started with the app's environment and the bundled page rendered.
    $app = Start-App @{}
    $hwnd = [IntPtr]::Zero
    $deadline = [DateTime]::UtcNow.AddMilliseconds($WindowTimeoutMs)
    while ($hwnd -eq [IntPtr]::Zero -and -not $app.HasExited -and [DateTime]::UtcNow -lt $deadline) {
        $hwnd = $W::FindTopLevel($app.Id, $product)
        if ($hwnd -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        if ($app.HasExited) { Stop-Fail "the installed app exited with $($app.ExitCode) before it showed a window" }
        Stop-Fail "the installed app showed no window titled $product"
    }
    if ($W::Class($hwnd) -eq '#32770') { Stop-Fail "the installed app couldn't start: $(Get-DialogText ($AE::FromHandle($hwnd)))" }
    $button = Find-Button $hwnd 'Begin test' $RenderTimeoutMs
    if (-not $button) { Stop-Fail "the page never rendered its Begin test button within $($RenderTimeoutMs / 1000) s" }
    Write-Host "the installed app rendered its Start screen (Begin test enabled: $($button.Current.IsEnabled))"
    Close-Window ($AE::FromHandle($hwnd))
    if (-not $app.WaitForExit($ExitTimeoutMs)) { Stop-Fail "the app did not exit within $($ExitTimeoutMs / 1000) s of its window closing" }
    Write-Host "the app exited with $($app.ExitCode)"

    # Checked after the app exits, so whatever it wrote at exit counts too.
    if (-not (Test-Path -LiteralPath $localData)) { Stop-Inconclusive "the app made no $localData, so WebView2 keeps its data elsewhere" }
    if ($PositiveControl -eq 'DataFile') { Set-Content -LiteralPath (Join-Path $localData 'planted.txt') 'synthetic' }
    $problems = @(Get-DataProblems $localData $roamingData)
    if ($problems) { Stop-Caught "the app left data outside WebView2's profile: $($problems -join '; ')" }
    Write-Host "$localData holds only EBWebView"

    # The loader honours WEBVIEW2_BROWSER_EXECUTABLE_FOLDER, and an empty folder looks like a machine
    # without the runtime.
    New-Item -ItemType Directory -Force $noRuntime | Out-Null
    $app = Start-App @{ WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = $noRuntime }
    $dialog = Find-Dialog $app $DialogTimeoutMs
    if (-not $dialog) {
        if ($app.HasExited) { Stop-Fail "without a runtime the app exited with $($app.ExitCode) and showed no dialog" }
        Stop-Fail "without a runtime the app showed no dialog within $($DialogTimeoutMs / 1000) s"
    }
    $text = Get-DialogText $dialog
    if ($dialog.Current.Name -ne $product -or $text -notlike '*WebView2 Runtime*') {
        Stop-Fail "the dialog is titled '$($dialog.Current.Name)' and reads '$text'"
    }
    # The box has only OK, so closing it returns OK. Its button offers no Invoke to pwsh's client.
    $buttons = @($dialog.FindAll($TreeScope::Descendants, $IsWin32Button))
    if ($buttons.Count -ne 1) { Stop-Fail "the dialog has $($buttons.Count) buttons, not one" }
    Close-Window $dialog
    if (-not $app.WaitForExit($ExitTimeoutMs)) { Stop-Fail 'the app did not exit after its dialog closed' }
    if ($app.ExitCode -ne 1) { Stop-Fail "after the missing-WebView2 dialog the app exited with $($app.ExitCode), not 1" }
    Write-Host "without a runtime the app said so and exited with 1: $text"

    if ($PositiveControl -ne 'Leftovers') { Invoke-Uninstaller }
    $left = @(Wait-Uninstalled $UninstallTimeoutMs)
    if ($left) { Stop-Caught "a silent uninstall left $($left -join ', ')" }
    $kept = @($localData, $productKey) | Where-Object { Test-Path -LiteralPath $_ }
    Write-Host "uninstalled; a silent uninstall never ticks 'Delete the application data', so it keeps: $(if ($kept) { $kept -join ', ' } else { 'nothing' })"

    if ($PositiveControl) { Stop-Fail "the $PositiveControl control was not caught" }
    Write-Host 'PASS: the installer installs, the app starts and renders, its data stays in WebView2''s profile, a missing runtime is explained, and the uninstaller removes the app'
    exit 0
}
catch {
    Stop-Inconclusive "the check failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    try {
        foreach ($proc in $apps) { if (-not $proc.HasExited) { $proc.Kill($true); [void]$proc.WaitForExit(5000) } }
        if (@(Get-Leftovers) -and (Test-Path -LiteralPath (Join-Path $installDir 'uninstall.exe'))) {
            Invoke-Uninstaller
            [void](Wait-Uninstalled $UninstallTimeoutMs)
        }
        # WebView2's processes outlive the app for a moment and hold files in its profile.
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            Remove-Item -LiteralPath $installDir, $localData, $roamingData, $noRuntime, $uninstallKey, $productKey -Recurse -Force -ErrorAction SilentlyContinue
            $rest = @($installDir, $localData, $roamingData, $noRuntime, $uninstallKey, $productKey) | Where-Object { Test-Path -LiteralPath $_ }
            if ($rest) { Start-Sleep -Milliseconds 500 }
        } while ($rest -and [DateTime]::UtcNow -lt $deadline)
        if ($rest) { Write-Host "could not remove: $($rest -join ', ')" }
        if ((Test-Path -LiteralPath $publisherKey) -and -not (Get-ChildItem -LiteralPath $publisherKey)) {
            Remove-Item -LiteralPath $publisherKey -ErrorAction SilentlyContinue
        }
    }
    catch { Write-Host "cleanup failed: $_" }
}
