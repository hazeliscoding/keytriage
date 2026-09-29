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
. (Join-Path $PSScriptRoot 'app-ui.ps1')

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

$apps = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()

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

    $app = Start-App $exe @{}
    $hwnd = Wait-StartScreen $app 'the installed app'
    Close-App $app $hwnd

    # Checked after the app exits, so whatever it wrote at exit counts too.
    if (-not (Test-Path -LiteralPath $localData)) { Stop-Inconclusive "the app made no $localData, so WebView2 keeps its data elsewhere" }
    if ($PositiveControl -eq 'DataFile') { Set-Content -LiteralPath (Join-Path $localData 'planted.txt') 'synthetic' }
    $problems = @(Get-DataProblems $localData $roamingData)
    if ($problems) { Stop-Caught "the app left data outside WebView2's profile: $($problems -join '; ')" }
    Write-Host "$localData holds only EBWebView"

    # The loader honours WEBVIEW2_BROWSER_EXECUTABLE_FOLDER, and an empty folder looks like a machine
    # without the runtime.
    New-Item -ItemType Directory -Force $noRuntime | Out-Null
    $app = Start-App $exe @{ WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = $noRuntime }
    $text = Read-ExitDialog $app 'without a runtime' '*WebView2 Runtime*'
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
