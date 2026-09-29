# Privacy

keytriage reads your keyboard to test it. This file lists each promise it makes, the code that keeps the promise, the check that proves it, and where the promise stops. Reviewed 2026.09.28, for v0.1.

## How to read this

- **Guards** read the source and the built app. They run in CI on every push, in the Network guard, Capture guard and Dependency guard jobs.
- **Tests** run in CI's Build (Windows) job: `cargo test --workspace` for Rust and `npm test` for the page.
- **App checks** run the app. The installer and connections checks run in the Build (Windows) job on every push. The focus, browser keys and crash reports checks take the foreground and need a debug build, so they run locally, with the commands in [AGENTS.md](AGENTS.md).
- Every check for something that must not happen has a positive control: a planted fault that the check must catch. A check that can't fail proves nothing.

## 1. Not a keylogger

keytriage reads keys only while a test runs and its window is in front.

Kept by:

- `register()` in `crates/input/src/raw_input.rs` registers Raw Input on the app's own window with flags `0` and no input sink, so Windows sends input only while the app owns the foreground.
- `subclass_proc()` in the same file closes a gate on `WM_ACTIVATE`, `WM_ACTIVATEAPP` and `WM_NCACTIVATE`, drops input queued before the switch, and unregisters.
- Capture exists only between `start_test` and the end of the test, in `src-tauri/src/test_session.rs`. `pause_test` drops it.
- `.device_event_filter(tauri::DeviceEventFilter::Always)` in `src-tauri/src/lib.rs` stops Tauri's own Raw Input registration.
- During a test the page cancels key events without reading which key they carried (`cancel()` in `src/app/keys.ts`).

Checked by:

- `scripts/check-capture.mjs` (CI) fails on input sinks, keyboard hooks, `GetAsyncKeyState`, `RegisterHotKey`, DirectInput background mode, Raw Input flags other than the named foreground ones, a filter other than `Always`, and known capture crates. It also scans the source of every crate the Windows build compiles. Its controls are in `scripts/check-capture.test.mjs`.
- `scripts/check-focus-capture.ps1` (local, in both WebView2 hosting modes) proves that keys arrive with the window in front, that none arrive with another process in front or during a pause, and that the process holds exactly one registration, with no sink. Its controls `Background`, `Registration` and `UserPause` must be caught.
- `src/app/privacy.spec.ts`: "cancels key events during a test without reading which key it was", with the control "catches a listener that reads the key".

Limits:

- tao, Tauri's window library, calls `GetAsyncKeyState` when the window gains focus, to see which keys are already down. It stores nothing. This was accepted on 2026.09.27, and `DEPENDENCY_ALLOWED` in `check-capture.mjs` pins it to that one call.
- Windows keeps its own keys during a test. The Windows key, Alt+F4, Print Screen, Ctrl+Alt+Del and the accessibility shortcuts still act.

## 2. The order of keys is never saved

An ordered list of keys is the text you typed, so it stays in memory and is gone when the test ends.

Kept by:

- A test's events live only in memory, in `src-tauri/src/test_session.rs`. When the test ends, only `Report::saved()` is kept, as the text of the export.
- `src-tauri/src/export.rs` names every field of the saved types, so a new field doesn't compile until it has had a privacy review. The file is written only through the Windows Save dialog, under a checked name.
- Types that hold timestamps or ordered events derive `Debug` only in tests, so they can't be printed into a log.
- The window is InPrivate (`"incognito": true` in `src-tauri/tauri.conf.json`), the app has no log plugin, and the page keeps its theme only in memory.

Checked by:

- `export.rs` tests `ns01` to `ns04`, which check that the file holds no list, no free text and nothing that depends on the order of keys, with `ns02_positive_control_catches_an_ordered_writer` as the control. `ex01` and `ex02` check the file name and every saved field.
- `privacy.spec.ts` "writes": through a whole test, a pause and a swap test to its export, the page writes nothing to local or session storage, IndexedDB, cookies, history or the console. One control per API must be caught.
- `golden_holds_only_the_synthetic_stream` and `swap_golden_holds_only_the_synthetic_stream` in `src-tauri/src/golden.rs` keep the test fixtures synthetic.

An exported report holds the app name, format, rules version, bin edges, a one-line note and the test's limits. Then, by scan code, for the keys the test prompted and the keys a finding names, it holds counts and histograms of hold times, release gaps and press intervals. It holds no key names, layout, dates, keyboard IDs or app version.

Limits:

- Hold and gap histograms describe how you press keys, which is biometric data (keystroke dynamics). Treat a report as personal.
- The report's file name holds the date and time of the test.
- A report attached to a public issue is public.

## 3. Keys are positions, not characters

The app works with scan codes. Letters exist only as labels on the drawn keyboard, and a report names keys by scan code.

## 4. The app makes no network connections

Kept by:

- There is no networking code, and no HTTP, updater, shell or opener plugin. The window's capabilities hold only `core:default`.
- The Content Security Policy in `src-tauri/tauri.conf.json` lets the page reach only the app's own IPC (`connect-src ipc: http://ipc.localhost`).
- `navigation_guard()` in `src-tauri/src/lib.rs` keeps the window on `http://tauri.localhost` and refuses every navigation during a test.
- Fonts are bundled. There are no update checks, and v0.1 opens no links.
- SmartScreen inside WebView2 is off, through `SetIsReputationCheckingRequired(false)` in `src-tauri/src/browser_ui.rs` and the `msSmartScreenProtection` flag in `src-tauri/src/crash_reports.rs`.

Checked by:

- `scripts/check-network.mjs` (CI) fails on web request APIs in the page's source and in the shipped bundle, on remote stylesheets, fonts and markup, a wider CSP, a missing navigation guard, Rust sockets, network crates, `windows` networking features and network plugins. Its controls are in `scripts/check-network.test.mjs`.
- `scripts/check-dependencies.mjs` (CI) fails on a direct dependency that isn't on its reviewed list. Its controls are in `scripts/check-dependencies.test.mjs`.
- `allows_the_bundled_app`, `allows_the_dev_server_only_in_dev` and `denies_everything_else` in `lib.rs` test the navigation guard's rule.
- `scripts/check-browser-keys.ps1` (local) reads the SmartScreen setting back as off.
- `scripts/check-connections.ps1` (CI) runs the release build under Windows Filtering Platform auditing and fails on any connection or bind by `keytriage.exe`. Its control, a stand-in process that connects, must be caught.

Limit: WebView2, which draws the window, makes its own connections. See section 7.

## 5. Crash output stays on this machine

A crash dump can hold what a process has in memory, including the events of a running test.

Kept by `src-tauri/src/crash_reports.rs`:

- WebView2 starts with custom crash reporting. Microsoft: "When `IsCustomCrashReportingEnabled` is set to `true`, Windows won't send crash data to the Microsoft endpoint."
- The app deletes WebView2's dumps at start, when WebView2 reports a failed process, and at exit.
- `SEM_NOGPFAULTERRORBOX` keeps the app's own crashes out of Windows Error Reporting.

Checked by `scripts/check-crash-reports.ps1` (local): five probes, and a control that turns all three off and must be caught by every probe.

Limits: a dump sits on disk until the sweep, and a crash of WebView2's browser process that Crashpad misses goes to Windows Error Reporting. Hang reports, dumps someone takes on purpose, an administrator's LocalDumps setting, the page and hibernation files, and system crash dumps are outside the app. The Crash output decision in [ROADMAP.md](ROADMAP.md) has the detail.

## 6. What keytriage leaves on your computer

- `%LOCALAPPDATA%\keytriage`: the app, `uninstall.exe` and the license files in `licenses\`.
- Its entry in Installed apps, and `HKCU\Software\Hazel Granados\keytriage`, where the installer remembers its folder.
- `%LOCALAPPDATA%\io.github.hazeliscoding.keytriage\EBWebView`: WebView2's own folder, with its browser profile, the components it downloads, such as certificate revocation lists, and Crashpad's settings. The page runs InPrivate and writes nothing there.
- The reports you export, wherever you save them.

Uninstalling removes the app. The data folder and the registry key go only when you tick "Delete the application data". A silent uninstall (`/S`) keeps them.

Checked by `scripts/check-installer.ps1` (CI, on a fresh runner): it installs, starts and uninstalls the app, checks that the data folder holds only `EBWebView`, and checks that there is no roaming data folder. Its controls `Leftovers` and `DataFile` must be caught.

## 7. What Microsoft's components do on their own

keytriage draws its window with Microsoft Edge WebView2, which Windows 11 includes. The runtime is Microsoft's, is shared with other apps, and talks to Microsoft whatever keytriage does. This section says what Microsoft documents. It doesn't promise that the runtime is silent.

- **Diagnostic data.** "Regardless of the Windows Diagnostic data setting, WebView2 collects required data that's necessary to maintain performance and reliability." Optional data, such as API usage and browser events, follows Settings > Privacy & security > Diagnostics & feedback > Diagnostic data. ([Data and privacy in WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy))
- **Experiments and configuration.** WebView2 downloads experiments and recommended settings from Microsoft's Experimentation and Configuration Service (`config.edge.skype.com`), which plays the role of Chromium's variations seed. That comparison is ours, not Microsoft's. "If you don't configure this policy on an unmanaged device, the behavior is the same as the 'FullMode'." The policy is `ExperimentationAndConfigurationServiceControl`, a DWORD under `HKLM\SOFTWARE\Policies\Microsoft\Edge\WebView2`: 1 fetches configuration only, and 0 stops the service, which Microsoft doesn't recommend. ([WebView2 policies](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-webview-policies), [Edge endpoints](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-security-endpoints))
- **Updates and components.** "The WebView2 Runtime updates automatically", through Microsoft Edge Update (`msedge.api.cdp.microsoft.com`, with downloads from `*.dl.delivery.mp.microsoft.com`). `edge.microsoft.com` provides "certificate revocation lists, and other browser component updates". ([Distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution), [Edge endpoints](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-security-endpoints))
- **Bugs.** WebView2 has made background requests that Microsoft treated as bugs: [#5047](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5047) and [#5093](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5093), fixed in 134, and [#2671](https://github.com/MicrosoftEdge/WebView2Feedback/issues/2671). What the runtime contacts changes between versions.
- **Measured.** On every push, the connections check lists what WebView2's processes contacted while the app sat 30 s on its Start screen, with the runtime version and date, in the Build (Windows) job's summary and its `connections` artifact. That is one runtime on a Windows Server runner, not your PC.
- **The installer** carries Microsoft's WebView2 bootstrapper, signed by Microsoft, which CI downloads from Microsoft over HTTPS when it builds the installer. It runs only when WebView2 is missing, and then "downloads and installs the Evergreen Runtime from Microsoft servers".
- **Windows** checks the installer you download with SmartScreen and, where it is on, Smart App Control. GitHub sees the download.

## 8. Check it yourself

- Compare the installer with the release's `SHA256SUMS.txt` (`Get-FileHash`, `certutil -hashfile` or `sha256sum -c`), and run `gh attestation verify <installer> --repo hazeliscoding/keytriage` to check that this repository's release workflow built it.
- Run the guards: `node scripts/check-network.mjs`, `node scripts/check-capture.mjs` and `node scripts/check-dependencies.mjs`.
- Run `scripts/check-connections.ps1 -Exe "$env:LOCALAPPDATA\keytriage\keytriage.exe"` from an elevated pwsh, or watch `keytriage.exe` in Resource Monitor's Network tab.
- Open an exported report. It is plain JSON.

## 9. Report a broken promise

A broken promise is a security bug. Report it privately through [Report a vulnerability](https://github.com/hazeliscoding/keytriage/security/advisories/new), never in a public issue. [SECURITY.md](SECURITY.md) says what is in scope and what to expect.
