#Requires -Version 7
# Browser keys check: proves that WebView2's own shortcuts and context menu do nothing in the app,
# and that the navigation guard refuses reloads during a test, so a key test can press every key.
# It reads the app's debug echo (src-tauri/src/echo.rs): the settings WebView2 reports, a line per
# page load as it starts, and a line per navigation the guard refuses. The echo starts a test once
# the page has loaded.
#  - The browser keys setting: F5, Ctrl+R and the Browser Refresh key must start no navigation at
#    all, and a right-click must open no menu window from the app's own WebView2 processes.
#  - The guard: F16 makes the page reload itself, which no setting stops, and the guard must refuse
#    it.
#  - SmartScreen: WebView2's reputation checking must read back as off.
# Ctrl+P is covered by the setting and left out, because a print dialog is hard to close.
#
# Build first with `npm run tauri build -- --debug --no-bundle`. The run takes the foreground and
# clicks the middle of the app window. Don't type while it runs.
#
# Exit codes: 0 pass, 1 the settings read back wrong or a positive control saw too little, 2
# inconclusive, 3 a browser key, the context menu or a reload acted. -PositiveControl BrowserKeys
# leaves the settings at the runtime's defaults, must read all three back as on and must catch its
# four probes, three in visual hosting, where WebView2 never reloads on Browser Refresh; -PositiveControl Reloads leaves the guard off and must catch the page's reload. Each
# exits 3 only through its own probes.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\debug\keytriage.exe'),
    [ValidateSet('windowed', 'visual')][string]$Hosting = 'windowed',
    [ValidateSet('', 'BrowserKeys', 'Reloads')][string]$PositiveControl = '',
    [int]$SettleMs = 3000,
    [int]$StartTimeoutMs = 60000
)
$ErrorActionPreference = 'Stop'

$Esc = 0x01; $Ctrl = 0x1D; $F5 = 0x3F; $F16 = 0x67; $VkR = 0x52; $BrowserRefresh = 0xA8

. (Join-Path $PSScriptRoot 'app-harness.ps1')

$app = $null
try {
    $switch = @{ '' = $null; 'BrowserKeys' = 'KEYTRIAGE_BROWSER_KEYS'; 'Reloads' = 'KEYTRIAGE_RELOADS' }[$PositiveControl]
    $app = [AppHarness.AppUnderTest]::Start((Resolve-Path $Exe), $Hosting -eq 'visual', $switch)
    Wait-TestStarted $StartTimeoutMs 10000
    $hwnd = Find-AppWindow $StartTimeoutMs
    if (-not $app.WaitFor({ $app.Settings -and $app.PageLoads -ge 1 }, $StartTimeoutMs)) {
        Stop-Inconclusive 'the app reported no WebView2 settings or no first page load'
    }
    Write-Host "settings: $($app.Settings)"
    $want = if ($PositiveControl -eq 'BrowserKeys') { 'browser-keys=1 context-menus=1 reputation-checks=1' } else { 'browser-keys=0 context-menus=0 reputation-checks=0' }
    if ($app.Settings -ne $want) { Stop-Fail "WebView2 reports $($app.Settings), not $want" }

    Enter-App $hwnd
    $R = $W::ScanFor($VkR, $hwnd)
    if ($R -eq 0) { Stop-Inconclusive 'the keyboard layout has no R key' }
    $setting = [System.Collections.Generic.List[string]]::new()
    $guard = [System.Collections.Generic.List[string]]::new()

    # Another window can take the foreground mid-probe, as a terminal did in development, and a
    # brief switch away pauses capture, so a key can go unseen. A probe then gets another try, up to
    # three. A caught action stands whatever happened around it, because neither can cause one. A
    # probe returns whether the browser acted, or 'unseen' when its input never reached the app.
    function Test-Probe([string]$name, [scriptblock]$probe) {
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            if ($W::GetForegroundWindow() -ne $hwnd) { Enter-App $hwnd }
            $pauses = $app.Pauses
            $result = & $probe
            if ($result -eq $true) { return $true }
            $fg = $W::GetForegroundWindow()
            if ($fg -ne $hwnd) { Write-Host "the foreground moved to $($W::Describe($fg)) during $name, trying again"; continue }
            if ($app.Pauses -ne $pauses) { Write-Host "capture paused during $name, trying again"; continue }
            if ($result -is [string]) { Stop-Inconclusive "$name never reached the app (UIPI or a secure desktop)" }
            return $false
        }
        Stop-Inconclusive "the foreground kept moving during $name"
    }

    # Whether the keys started a navigation: a page load, or a reload the guard refused. The app's
    # own capture sees each key down, which proves the keys arrived.
    function Test-Navigation([int]$downs, [scriptblock]$send) {
        $loads = $app.PageLoads
        $refused = $app.Refusals
        $seen = $app.OtherDowns
        [void](& $send)
        if (-not $app.WaitFor({ $app.OtherDowns -ge $seen + $downs }, $SettleMs)) { return 'unseen' }
        return $app.WaitFor({ $app.PageLoads -gt $loads -or $app.Refusals -gt $refused }, $SettleMs)
    }

    if (Test-Probe 'F5' { Test-Navigation 1 { $W::Tap($F5, 1) } }) { $setting.Add('F5 started a reload') }
    if (Test-Probe 'Ctrl+R' { Test-Navigation 2 { $W::Chord($Ctrl, $R) } }) { $setting.Add('Ctrl+R started a reload') }
    if (Test-Probe 'Browser Refresh' { Test-Navigation 1 { $W::TapVirtualKey($BrowserRefresh) } }) { $setting.Add('the Browser Refresh key started a reload') }

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
    if ($menu) { $setting.Add('a right-click opened a context menu') }

    # The page reloads itself on F16. The guard must refuse it, and a refusal must show up, or the
    # probe proved nothing.
    $refused = $app.Refusals
    $reloaded = Test-Probe 'F16' {
        $loads = $app.PageLoads
        $seen = $app.OtherDowns
        [void]$W::Tap($F16, 1)
        if (-not $app.WaitFor({ $app.OtherDowns -ge $seen + 1 }, $SettleMs)) { return 'unseen' }
        [void]$app.WaitFor({ $app.PageLoads -gt $loads -or $app.Refusals -gt $refused }, $SettleMs)
        $app.PageLoads -gt $loads
    }
    if ($reloaded) { $guard.Add('the page reloaded itself during a test') }
    elseif ($app.Refusals -le $refused) { Stop-Inconclusive 'the page''s own reload never reached the navigation guard' }

    switch ($PositiveControl) {
        'BrowserKeys' {
            $setting | ForEach-Object { Write-Host "caught: $_" }
            $required = [System.Collections.Generic.List[string]]@('F5 started a reload', 'Ctrl+R started a reload', 'a right-click opened a context menu')
            # In visual hosting WebView2 never reloads on the Browser Refresh key, even with its
            # shortcuts on (runtime 153, 2026.09.29), so the control can't ask for it there. The
            # navigation guard still refuses every reload during a test, which the F16 probe proves.
            if ($Hosting -eq 'windowed') { $required.Add('the Browser Refresh key started a reload') }
            $missed = @($required | Where-Object { $setting -notcontains $_ })
            if ($missed.Count) { Stop-Fail "with the browser keys left on, these probes caught nothing: $($missed -join '; ')" }
            Write-Host 'FAIL (positive control): every browser keys probe it needs caught its action'
            exit 3
        }
        'Reloads' {
            $guard | ForEach-Object { Write-Host "caught: $_" }
            if ($guard.Count -lt 1) { Stop-Fail 'with the guard off, the page''s reload was still refused' }
            Write-Host 'FAIL (positive control): the guard probe caught the reload'
            exit 3
        }
    }
    $failures = @($setting) + @($guard)
    if ($failures.Count -gt 0) {
        $failures | ForEach-Object { Write-Host "FAIL: $_" }
        exit 3
    }
    Write-Host "PASS ($Hosting hosting): F5, Ctrl+R and Browser Refresh started no navigation, a right-click opened no menu, and the guard refused the page's reload"
    exit 0
}
catch {
    Stop-Inconclusive "the harness failed: $_"
}
finally {
    # A throw here would replace the exit code above.
    if ($app) { try { $app.Dispose() } catch {} }
}
