#Requires -Version 7
# Portable check: unzips the portable zip into a fresh folder and checks what it holds, starts the
# app and finds its rendered Begin test button, checks that the app's data stays in the folder
# beside the exe, checks that a copy in a folder it can't write says so and exits 1, then deletes
# the folder and checks that nothing is left.
#
# The app it starts keeps its data in %LOCALAPPDATA% when the marker is missing, as the installed
# app does, so it runs only on a CI runner (GITHUB_ACTIONS=true), or with -AllowLocal in a throwaway
# VM. It stops before starting when keytriage is or was installed, and afterwards removes what its
# run created.
#
# Exit codes: 0 pass, 1 fail, 2 inconclusive, 3 data outside the unzipped folder or left behind,
# which the check exists to catch. Each positive control must exit 3: -PositiveControl NoMarker
# deletes the marker, so the app keeps its data in %LOCALAPPDATA%, DataFile plants a file in the
# data folder before it is checked, and Leftovers skips deleting the folder.
param(
    [Parameter(Mandatory)][string]$Zip,
    [ValidateSet('NoMarker', 'DataFile', 'Leftovers')][string]$PositiveControl,
    [switch]$AllowLocal,
    [int]$WindowTimeoutMs = 30000,
    [int]$RenderTimeoutMs = 60000,
    [int]$DialogTimeoutMs = 20000,
    [int]$ExitTimeoutMs = 10000,
    [int]$DeleteTimeoutMs = 30000
)
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'app-harness.ps1')

if ($env:GITHUB_ACTIONS -ne 'true' -and -not $AllowLocal) {
    Stop-Inconclusive 'without its marker keytriage keeps its data where an installed copy does, so this runs only on a CI runner, or with -AllowLocal in a throwaway VM'
}
. (Join-Path $PSScriptRoot 'app-ui.ps1')

$config = Get-Content (Join-Path $PSScriptRoot '..\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$overlay = Get-Content (Join-Path $PSScriptRoot '..\src-tauri\tauri.release.conf.json') -Raw | ConvertFrom-Json
$product = $config.productName
$publisher = $config.bundle.publisher
# Empty names would point the cleanup at %LOCALAPPDATA% itself.
if (-not $product -or -not $publisher -or -not $config.identifier) { Stop-Inconclusive 'tauri.conf.json lacks productName, identifier or bundle.publisher' }
$file = Split-Path -Leaf $Zip
if ($file -notmatch "^$([regex]::Escape($product))_(\d+\.\d+\.\d+)_x64-portable\.zip$") {
    Stop-Inconclusive "$file is not named ${product}_X.Y.Z_x64-portable.zip"
}
$version = $Matches[1]
if (-not (Test-Path -LiteralPath $Zip -PathType Leaf)) { Stop-Inconclusive "no zip at $Zip" }
$Zip = (Resolve-Path -LiteralPath $Zip).Path

# The names src-tauri/src/portable.rs gives the marker and the data folder. The zip holds one
# folder with the exe, the marker and the license files the installer ships.
$marker = 'keytriage.portable'
$dataName = 'keytriage-data'
$licenses = @($overlay.bundle.resources.PSObject.Properties.Value)
if (-not $licenses) { Stop-Inconclusive 'tauri.release.conf.json names no license files' }
$contents = @("$product.exe", $marker) + $licenses

# Where the installed app keeps its data and NSIS remembers its folder. The portable app writes none
# of them.
$localData = Join-Path $env:LOCALAPPDATA $config.identifier
$roamingData = Join-Path $env:APPDATA $config.identifier
$productKey = "HKCU:\Software\$publisher\$product"
$elsewhere = @($localData, $roamingData, $productKey)
$present = $elsewhere | Where-Object { Test-Path -LiteralPath $_ }
if ($present) { Stop-Inconclusive "keytriage is or was installed here: $($present -join ', ')" }

# The spaces check that no step splits the path.
$temp = [IO.Path]::GetTempPath()
$root = Join-Path $temp "keytriage portable $PID"
$locked = Join-Path $temp "keytriage read-only $PID"
if ((Test-Path -LiteralPath $root) -or (Test-Path -LiteralPath $locked)) { Stop-Inconclusive "$root or $locked already exists" }
$folder = Join-Path $root $product
$exe = Join-Path $folder "$product.exe"
$data = Join-Path $folder $dataName
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

$apps = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()

function Get-Elsewhere { $elsewhere | Where-Object { Test-Path -LiteralPath $_ } }

# WebView2 keeps its profile in EBWebView. Anything else beside the exe, such as WebView2's default
# keytriage.exe.WebView2 folder, is data the privacy contract doesn't account for.
function Get-FolderProblems {
    $expected = @("$product.exe", $marker, 'licenses', $dataName)
    Get-ChildItem -LiteralPath $folder -Force | Where-Object Name -notin $expected | ForEach-Object { "$folder holds $($_.Name)" }
    Get-ChildItem -LiteralPath (Join-Path $folder 'licenses') -Force | Where-Object { "licenses/$($_.Name)" -notin $licenses } | ForEach-Object { "licenses holds $($_.Name)" }
    Get-ChildItem -LiteralPath $data -Force | Where-Object Name -ne 'EBWebView' | ForEach-Object { "$data holds $($_.Name)" }
}

# WebView2's processes outlive the app for a moment and hold files in its profile.
function Remove-Folder([string]$path, [int]$timeoutMs) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction SilentlyContinue
        $left = Test-Path -LiteralPath $path
        if ($left) { Start-Sleep -Milliseconds 500 }
    } while ($left -and [DateTime]::UtcNow -lt $deadline)
    return -not $left
}

try {
    # .NET reads the names as stored, so a backslash or a stray entry shows here. Folders need no
    # entries of their own.
    $archive = [IO.Compression.ZipFile]::OpenRead($Zip)
    try { $entries = @($archive.Entries | ForEach-Object FullName | Where-Object { -not $_.EndsWith('/') } | Sort-Object) }
    finally { $archive.Dispose() }
    $wanted = @($contents | ForEach-Object { "$product/$_" } | Sort-Object)
    if (($entries -join "`n") -cne ($wanted -join "`n")) { Stop-Fail "the zip holds $($entries -join ', '), not $($wanted -join ', ')" }

    [IO.Compression.ZipFile]::ExtractToDirectory($Zip, $root)
    $empty = @($contents | Where-Object { $_ -ne $marker -and (Get-Item -LiteralPath (Join-Path $folder $_)).Length -eq 0 })
    if ($empty) { Stop-Fail "the zip holds empty files: $($empty -join ', ')" }
    $info = (Get-Item -LiteralPath $exe).VersionInfo
    if ($info.CompanyName -ne $publisher -or $info.ProductVersion -ne $version) {
        Stop-Fail "the exe's version info reads CompanyName '$($info.CompanyName)' and ProductVersion '$($info.ProductVersion)'"
    }
    Write-Host "unzipped $version into $folder, with the marker and $($licenses.Count) license files"
    if ($PositiveControl -eq 'NoMarker') { Remove-Item -LiteralPath (Join-Path $folder $marker) }

    $app = Start-App $exe @{}
    $hwnd = Wait-StartScreen $app 'the portable app'
    Close-App $app $hwnd

    # Checked after the app exits, so whatever it wrote at exit counts too.
    $outside = @(Get-Elsewhere)
    if ($outside) { Stop-Caught "the app wrote outside its folder: $($outside -join ', ')" }
    if (-not (Test-Path -LiteralPath (Join-Path $data 'EBWebView'))) { Stop-Fail "the app made no $data\EBWebView, so WebView2 keeps its data elsewhere" }
    if ($PositiveControl -eq 'DataFile') { Set-Content -LiteralPath (Join-Path $data 'planted.txt') 'synthetic' }
    $problems = @(Get-FolderProblems)
    if ($problems) { Stop-Caught "the app left data outside WebView2's profile: $($problems -join '; ')" }
    Write-Host "$data holds only EBWebView, and none of $($elsewhere -join ', ') exists"

    # As under Program Files for a standard user. A deny entry for the user holds for the runner's
    # administrator too, who can write to Program Files.
    New-Item -ItemType Directory $locked | Out-Null
    Copy-Item -LiteralPath $exe, (Join-Path $folder $marker) -Destination $locked
    & icacls.exe $locked /deny "*${sid}:(OI)(CI)(WD,AD)" | Out-Null
    if ($LASTEXITCODE -ne 0) { Stop-Inconclusive "icacls could not deny writes to $locked" }
    $app = Start-App (Join-Path $locked "$product.exe") @{}
    $text = Read-ExitDialog $app 'in a folder it can''t write,' "*can't write to *Move the folder that holds $product.exe*"
    # The runner's %TEMP% can be an 8.3 path, which the app may spell out, so only the end is matched.
    if (-not $text.Contains("$(Split-Path -Leaf $locked)\$dataName.")) { Stop-Fail "the dialog doesn't name $locked\$dataName`: $text" }
    $outside = @(Get-Elsewhere)
    if ($outside) { Stop-Caught "in a folder it can't write, the app wrote outside it: $($outside -join ', ')" }
    Write-Host "in a folder it can't write, the app said so and exited with 1: $text"

    if ($PositiveControl -ne 'Leftovers' -and -not (Remove-Folder $folder $DeleteTimeoutMs)) {
        Write-Host "the folder could not be deleted within $($DeleteTimeoutMs / 1000) s"
    }
    $left = @(@($folder) + $elsewhere | Where-Object { Test-Path -LiteralPath $_ })
    if ($left) { Stop-Caught "deleting the folder left $($left -join ', ')" }
    Write-Host "deleted $folder, and nothing is left"

    if ($PositiveControl) { Stop-Fail "the $PositiveControl control was not caught" }
    Write-Host 'PASS: the zip holds the app, its marker and its licenses, the app renders and keeps its data beside the exe, a folder it can''t write is explained, and deleting the folder leaves nothing'
    exit 0
}
catch {
    Stop-Inconclusive "the check failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    try {
        foreach ($proc in $apps) { if (-not $proc.HasExited) { $proc.Kill($true); [void]$proc.WaitForExit(5000) } }
        if (Test-Path -LiteralPath $locked) { & icacls.exe $locked /remove:d "*$sid" /T /C /Q | Out-Null }
        # None of these existed when the check began.
        $created = @($root, $locked) + $elsewhere
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            Remove-Item -LiteralPath $created -Recurse -Force -ErrorAction SilentlyContinue
            $rest = @($created | Where-Object { Test-Path -LiteralPath $_ })
            if ($rest) { Start-Sleep -Milliseconds 500 }
        } while ($rest -and [DateTime]::UtcNow -lt $deadline)
        if ($rest) { Write-Host "could not remove: $($rest -join ', ')" }
    }
    catch { Write-Host "cleanup failed: $_" }
}
