#Requires -Version 7
# Connections check: proves that keytriage.exe opens no network connection, and measures what
# WebView2's processes contact on their own. For the run it turns on Windows Filtering Platform
# auditing (the Filtering Platform Connection subcategory) and the DNS-Client log, and afterwards
# restores both. It starts the app, leaves it idle on the Start screen, closes it, and reads the
# audit events.
#
# Asserted: no permitted or blocked connection (5156, 5157) and no bind (5158) by keytriage.exe.
# Measured, never asserted: the connections of the WebView2 processes the app started, with host
# names where the DNS-Client log ties them to those processes. They change with runtime versions.
#
# It needs an elevated pwsh, because it changes the audit policy and reads the Security log. CI runs
# it on the release build.
#
# Exit codes: 0 pass, 1 the positive control was not caught, 2 inconclusive, 3 keytriage.exe made a
# connection. -PositiveControl watches a stand-in pwsh process that connects to a loopback listener
# this script opens, and must exit 3.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\keytriage.exe'),
    [int]$Seconds = 30,
    [string]$Out,
    [switch]$PositiveControl,
    [int]$StartTimeoutMs = 30000,
    [int]$ExitTimeoutMs = 10000
)
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'app-harness.ps1')

# Filtering Platform Connection, by GUID, so auditpol's arguments are the same in every language.
$Subcategory = '{0CCE9226-69AE-11D9-BED3-505054503030}'
$DnsLogName = 'Microsoft-Windows-DNS-Client/Operational'
$Protocols = @{ '1' = 'ICMP'; '6' = 'TCP'; '17' = 'UDP'; '58' = 'ICMPv6' }
$Directions = @{ '%%14592' = 'inbound'; '%%14593' = 'outbound' }

function ConvertFrom-AuditEvent([int]$id, [string]$xml) {
    # Hashtable keys ignore case: 5156 and 5157 name the field ProcessID, 5158 ProcessId.
    $data = @{}
    foreach ($d in ([xml]$xml).Event.EventData.Data) { $data[$d.Name] = $d.'#text' }
    $protocol = $Protocols[[string]$data['Protocol']]
    [pscustomobject]@{
        Event       = $id
        ProcessId   = [int]$data['ProcessID']
        Application = $data['Application']
        Direction   = if ($id -eq 5158) { 'bind' } else { $Directions[[string]$data['Direction']] }
        Protocol    = if ($protocol) { $protocol } else { [string]$data['Protocol'] }
        Address     = if ($id -eq 5158) { $data['SourceAddress'] } else { $data['DestAddress'] }
        Port        = if ($id -eq 5158) { $data['SourcePort'] } else { $data['DestPort'] }
    }
}

# DNS-Client's QueryResults read like "type:  5 alias.example;::ffff:192.0.2.1;2001:db8::1;".
function Get-ResultAddresses([string]$results) {
    foreach ($part in ($results -split ';')) {
        $ip = $null
        if ([System.Net.IPAddress]::TryParse($part.Trim(), [ref]$ip)) {
            if ($ip.IsIPv4MappedToIPv6) { $ip = $ip.MapToIPv4() }
            $ip.ToString()
        }
    }
}

function Get-ProcessRole([string]$commandLine) {
    if ($commandLine -notmatch '--type=(\S+)') { return 'browser' }
    $type = $Matches[1]
    if ($type -eq 'utility' -and $commandLine -match '--utility-sub-type=(\S+)') { return "utility $($Matches[1])" }
    return $type
}

function Format-Summary($report) {
    $lines = @(
        "### What WebView2 contacted (measured, not asserted)", '',
        "Runtime $($report.runtime) on $($report.os), $($report.date), $($report.seconds) s idle on the Start screen. keytriage.exe made no connection.", ''
    )
    if ($report.webview2.Count -eq 0) { return $lines + 'No connection by the WebView2 processes was audited.' }
    $lines += '| Process | Direction | Protocol | Address | Port | Host | Events |', '| --- | --- | --- | --- | --- | --- | --- |'
    foreach ($r in $report.webview2) {
        $lines += "| $($r.process) | $($r.direction) | $($r.protocol) | $($r.address) | $($r.port) | $($r.host) | $($r.events) |"
    }
    return $lines
}

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Stop-Inconclusive 'this needs an elevated pwsh: it changes the audit policy and reads the Security log' }
try { Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes }
catch { Stop-Inconclusive "UI Automation did not load: $_" }
if (-not $PositiveControl -and -not (Test-Path -LiteralPath $Exe -PathType Leaf)) { Stop-Inconclusive "no app at $Exe" }

# The backup's Setting Value is 0 none, 1 success, 2 failure, 3 both. Its text columns are
# translated, so the GUID and the number are read by position.
function Get-AuditSetting {
    $file = Join-Path ([IO.Path]::GetTempPath()) "keytriage-auditpol-$PID.csv"
    try {
        auditpol /backup "/file:$file" | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "auditpol /backup exited with $LASTEXITCODE" }
        $rows = @(Get-Content -LiteralPath $file | Select-Object -Skip 1 |
                ConvertFrom-Csv -Header Machine, Target, Name, Guid, Inclusion, Exclusion, Value |
                Where-Object Guid -match '^\{[0-9A-F-]{36}\}$')
        if ($rows.Count -eq 0) { throw 'auditpol /backup listed no subcategories' }
        $row = $rows | Where-Object Guid -eq $Subcategory
        # A subcategory missing from the backup has no auditing.
        if ($row) { [int]$row.Value } else { 0 }
    }
    finally { Remove-Item -LiteralPath $file -Force -ErrorAction SilentlyContinue }
}

function Set-AuditSetting([int]$value) {
    $success = if ($value -band 1) { 'enable' } else { 'disable' }
    $failure = if ($value -band 2) { 'enable' } else { 'disable' }
    auditpol /set "/subcategory:$Subcategory" "/success:$success" "/failure:$failure" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "auditpol /set exited with $LASTEXITCODE" }
}

$tree = [System.Collections.Generic.HashSet[uint32]]::new()
$roles = @{}
$runtime = $null
function Update-Tree([int]$root) {
    foreach ($id in $W::ProcessTree([uint32]$root)) {
        if (-not $tree.Add($id) -or $id -eq $root) { continue }
        $p = Get-CimInstance Win32_Process -Filter "ProcessId = $id" -ErrorAction SilentlyContinue
        if (-not $p) { continue }
        $roles[[int]$id] = "$($p.Name) $(Get-ProcessRole $p.CommandLine)"
        if (-not $script:runtime -and $p.Name -eq 'msedgewebview2.exe' -and $p.ExecutablePath) {
            $script:runtime = (Get-Item -LiteralPath $p.ExecutablePath).VersionInfo.ProductVersion
        }
    }
}

# Samples the process tree every 500 ms, so WebView2's processes are known by id, until `done`.
function Wait-Sampling([int]$root, [int]$timeoutMs, [scriptblock]$done) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
    do {
        Update-Tree $root
        if (& $done) { return $true }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    return [bool](& $done)
}

function Start-Watched([string]$file, [string]$arguments) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($file, $arguments)
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    # WebView2 reads these, and a runner's own could move the data folder or pick another runtime.
    foreach ($name in @($psi.Environment.Keys)) {
        if ($name -like 'WEBVIEW2_*' -or $name -like 'COREWEBVIEW2_*') { [void]$psi.Environment.Remove($name) }
    }
    [System.Diagnostics.Process]::Start($psi)
}

$savedAudit = $null
$dnsLog = $null
$dnsWasOn = $true
$proc = $null
$listeners = @()
try {
    $savedAudit = Get-AuditSetting
    Set-AuditSetting 3
    $dnsLog = [System.Diagnostics.Eventing.Reader.EventLogConfiguration]::new($DnsLogName)
    $dnsWasOn = $dnsLog.IsEnabled
    if (-not $dnsWasOn) { $dnsLog.IsEnabled = $true; $dnsLog.SaveChanges() }
    $since = (Get-Date).AddSeconds(-1)

    if ($PositiveControl) {
        # A loopback listener, so the control touches no network and raises no firewall prompt. The
        # socket lives only in this control, never in the app.
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
        $listener.Start()
        $listeners += $listener
        $port = $listener.LocalEndpoint.Port
        $connect = "`$c = [System.Net.Sockets.TcpClient]::new(); `$c.Connect('127.0.0.1', $port); `$c.Close()"
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($connect))
        $proc = Start-Watched (Get-Process -Id $PID).Path "-NoProfile -NonInteractive -EncodedCommand $encoded"
        if (-not (Wait-Sampling $proc.Id $ExitTimeoutMs { $proc.HasExited })) { Stop-Inconclusive 'the stand-in did not finish' }
        if ($proc.ExitCode -ne 0 -or -not $listener.Pending()) { Stop-Inconclusive "the stand-in did not connect to 127.0.0.1:$port" }
        Write-Host "the stand-in (pid $($proc.Id)) connected to 127.0.0.1:$port"
    }
    else {
        $proc = Start-Watched (Resolve-Path -LiteralPath $Exe).Path ''
        $script:hwnd = [IntPtr]::Zero
        $found = Wait-Sampling $proc.Id $StartTimeoutMs {
            $script:hwnd = $W::FindTopLevel($proc.Id, 'keytriage')
            $script:hwnd -ne [IntPtr]::Zero -or $proc.HasExited
        }
        if (-not $found -or $proc.HasExited) { Stop-Inconclusive 'the app showed no window titled keytriage' }
        Write-Host "the app is up; idling $Seconds s"
        [void](Wait-Sampling $proc.Id ($Seconds * 1000) { $false })
        # Process.CloseMainWindow can pick another window the process owns, such as the input
        # indicator's.
        [System.Windows.Automation.AutomationElement]::FromHandle($script:hwnd).GetCurrentPattern(
            [System.Windows.Automation.WindowPattern]::Pattern).Close()
        if (-not (Wait-Sampling $proc.Id $ExitTimeoutMs { $proc.HasExited })) { Stop-Fail 'the app did not exit after its window closed' }
    }
    # The audit is written a moment after the fact.
    Start-Sleep -Seconds 3

    $events = @(Get-WinEvent -FilterHashtable @{ LogName = 'Security'; Id = 5156, 5157, 5158; StartTime = $since } -ErrorAction SilentlyContinue |
            ForEach-Object { ConvertFrom-AuditEvent $_.Id $_.ToXml() })
    if ($events.Count -eq 0) { Stop-Inconclusive 'the audit recorded no connection by any process, so it is not working' }
    $watched = @($events | Where-Object ProcessId -eq $proc.Id)
    $describe = { "$($_.Direction) $($_.Protocol) $($_.Address):$($_.Port) (event $($_.Event))" }

    if ($PositiveControl) {
        if ($watched.Count -eq 0) { Stop-Fail "the stand-in's connections were not audited, so the check can't see one" }
        $caught = $watched | ForEach-Object $describe
        if ($env:GITHUB_STEP_SUMMARY) {
            Add-Content $env:GITHUB_STEP_SUMMARY "Connections check's positive control caught the stand-in: $($caught -join '; ')."
        }
        Stop-Caught "the stand-in's connections were caught: $($caught -join '; ')"
    }

    $webviewIds = @($tree | Where-Object { $_ -ne $proc.Id } | ForEach-Object { [int]$_ })
    $dns = @(Get-WinEvent -FilterHashtable @{ LogName = $DnsLogName; Id = 3006, 3008; StartTime = $since } -ErrorAction SilentlyContinue |
            Where-Object { $webviewIds -contains $_.ProcessId })
    $hosts = @{}
    foreach ($e in $dns | Where-Object Id -eq 3008) {
        $data = @{}
        foreach ($d in ([xml]$e.ToXml()).Event.EventData.Data) { $data[$d.Name] = $d.'#text' }
        foreach ($a in Get-ResultAddresses $data['QueryResults']) { $hosts[$a] = $data['QueryName'] }
    }
    $queried = @($dns | ForEach-Object { ([xml]$_.ToXml()).Event.EventData.Data | Where-Object Name -eq 'QueryName' | ForEach-Object '#text' } |
            Sort-Object -Unique)

    $rows = @($events | Where-Object { $webviewIds -contains $_.ProcessId } |
            Group-Object { $roles[$_.ProcessId] }, Direction, Protocol, Address, Port | ForEach-Object {
                $first = $_.Group[0]
                [ordered]@{
                    process   = $roles[$first.ProcessId]
                    direction = $first.Direction
                    protocol  = $first.Protocol
                    address   = $first.Address
                    port      = $first.Port
                    host      = if ($hosts[$first.Address]) { $hosts[$first.Address] } else { '(no DNS-Client event names it)' }
                    events    = $_.Count
                }
            })
    $os = Get-CimInstance Win32_OperatingSystem
    $report = [ordered]@{
        date       = Get-Date -Format 'yyyy.MM.dd'
        os         = "$($os.Caption) $($os.Version)"
        runtime    = if ($runtime) { $runtime } else { 'unknown' }
        seconds    = $Seconds
        run        = if ($env:GITHUB_RUN_ID) { "$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID" } else { $null }
        keytriage  = @($watched | ForEach-Object $describe)
        webview2   = $rows
        dnsQueries = $queried
        processes  = @($webviewIds | ForEach-Object { $roles[$_] } | Where-Object { $_ } | Sort-Object -Unique)
    }
    if ($Out) {
        New-Item -ItemType Directory -Force (Split-Path -Parent ([IO.Path]::GetFullPath($Out))) | Out-Null
        $report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Out
    }

    if ($watched.Count -gt 0) {
        $watched | ForEach-Object $describe | ForEach-Object { Write-Host "FAIL: keytriage.exe: $_" }
        Stop-Caught "keytriage.exe (pid $($proc.Id)) made $($watched.Count) connection or bind event(s)"
    }
    $summary = Format-Summary $report
    $summary | ForEach-Object { Write-Host $_ }
    if ($env:GITHUB_STEP_SUMMARY) { $summary | Add-Content $env:GITHUB_STEP_SUMMARY }
    Write-Host "PASS: keytriage.exe made no connection in $Seconds s on the Start screen; its $($webviewIds.Count) WebView2 processes made $($rows.Count) kinds of connection"
    exit 0
}
catch {
    Stop-Inconclusive "the check failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    try { if ($proc -and -not $proc.HasExited) { $proc.Kill($true) } } catch { Write-Host "could not stop the watched process: $_" }
    foreach ($l in $listeners) { try { $l.Stop() } catch {} }
    try { if ($null -ne $savedAudit) { Set-AuditSetting $savedAudit } } catch { Write-Host "could not restore the audit policy: $_" }
    try { if ($dnsLog -and -not $dnsWasOn) { $dnsLog.IsEnabled = $false; $dnsLog.SaveChanges() } } catch { Write-Host "could not restore the DNS-Client log: $_" }
}
