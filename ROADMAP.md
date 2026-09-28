# Roadmap

keytriage is a local-first desktop app (Rust + Tauri v2 + Angular, Windows first) that diagnoses keyboard faults from evidence and guides the tests that isolate them. This file tracks what gets built, in what order, and the decisions already made.

## Decisions (2026.09.25)

- **Position:** a diagnostic and repair assistant, not another keyboard tester. Testers exist (chatter-cli, HIDTester, KeyboardTest.tech), and so do chatter filters (Keyboard Chatter Blocker) and keyboard databases (SonixQMK, QMK, VIA). Nothing yet goes from a symptom to its likely cause and then to the repair. keytriage reuses those sources rather than rebuilding them.
- **Evidence, not verdicts.** Every finding carries its evidence, a confidence level (Low, Medium, High, Very high), alternative causes and a recommended next test. Scores can exist internally, but users see words. Never say a part is broken.
- **The swap test is the core workflow.** On a hot-swap board, swap the suspect switch with a known-good one and test both keys again. If the fault moves with the switch, the switch is the cause. If it stays, the socket, PCB or matrix is. Soldered and laptop keyboards get different next steps. v0.1 asks the user which kind of board they have.
- **Stack:**
  - Rust core, Tauri v2 desktop app, Angular UI.
  - Windows first, because Raw Input tells us which keyboard sent each event.
  - A Rust workspace with `crates/input` (the OS layer) and `crates/diagnostics` (pure Rust, no OS or Tauri code), so Linux and a CLI can reuse the engine.
- **Keys are physical positions** (scancodes), never characters. Layouts are generic ANSI and ISO boards until identification lands.
- **Capture only in the foreground.**
  - Raw Input is registered on the app's own window, with no input sink. It receives input only while the app is focused and a test is running.
  - No low-level keyboard hooks.
  - Tauri's own Raw Input registration is off (`DeviceEventFilter::Always`). tao otherwise registers every keyboard at startup, and `Never` adds an input sink.
  - A CI check fails on `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx` and `WH_KEYBOARD_LL`, with a positive control.
- **Only aggregates are saved.** The ordered event stream lives in memory for the live view and the running test. Reports and anything else written to disk hold per-key counts and timing statistics only, because an ordered list of keys is the text that was typed.
- **No network in the app.**
  - There is no networking code and no auto-updater.
  - The Content Security Policy blocks outbound requests from the page, and a navigation guard keeps the window on the app.
  - A CI check fails on web request APIs and HTTP crates, with a positive control.
  - Data such as prices ships inside the app, and links open in the browser.
- **Prices come from a CI snapshot.** A scheduled GitHub workflow reads public retailer product feeds, such as Shopify's `/products.json`. It respects robots.txt and each shop's terms, and commits a price and stock file with an "as of" date. The app bundles that file. Prices refresh with releases.
- **No affiliate links.** Parts links are plain and do not earn money.
- **Deterministic diagnosis.** Measurements go through rules to findings. No LLM in the diagnosis path.
- **Honest limits.** Software sees what the firmware reports after its own debounce, and timing resolution depends on the keyboard's polling rate. Reports say so where it matters.
- **The UI design comes from the owner.** Beyond the M0 placeholder, no UI code (templates, styles or layout) until that design is in `docs/design/`.
- **License:** Apache-2.0. The repo is public.
- **Brand** is option 2a, "Frame" (2026.09.26, replacing "Focus"): two heavy corner jaws holding a single red key. The wordmark reads `KEY//TRIAGE` in Barlow Semi Condensed SemiBold, uppercase, with 0.08em letter spacing, converted to vector paths. Ink is `#1b1812`, and the key and the slashes use the red accent `#9e2b2b`. The dark variants use `#e8e1d2` and `#b5383a`. The assets are in `docs/brand/`.
- **Scaffold** (2026.09.26): `crates/diagnostics` forbids `unsafe` code, which keeps FFI out of the engine. The Tauri template's log plugin is left out, because it writes log files to disk.
- **Guards** (2026.09.27): the network guard also scans the built bundle, which holds npm code, our Rust for sockets, and the `windows` crates for networking features. The capture guard also bans `GetAsyncKeyState`, `RegisterHotKey` and DirectInput background mode, allows Raw Input flags only from the named foreground flags, and fails on known hook and hotkey crates. Both match names, not behavior, so a crate that connects or captures under an unlisted name would get past them. A third guard therefore fails on any direct dependency that is not on its crate's allowlist in `scripts/check-dependencies.mjs`, so each new crate is a reviewed choice. Transitive crates are left to the name checks, which keeps upgrades free of list churn.
- **Raw Input** (2026.09.27):
  - **How capture works.** It registers the keyboard (usage page 1, usage 6) with flags `0`, targets the main window, and reads `WM_INPUT` in a comctl32 subclass on that window (`crates/input`).
  - **Foreground only.** Windows delivers the input only while our process owns the foreground window. That includes the time keyboard focus sits in WebView2's child window inside `msedgewebview2.exe`.
  - **The target is never null.** A null target follows keyboard focus into WebView2's process.
  - **The check.** `scripts/check-focus-capture.ps1` runs against a debug build and proves three things:
    - keys arrive while focus is inside WebView2's process;
    - none arrive while another process's window is in front;
    - the process holds exactly one registration, with no sink flag.

    It also passes with `-Hosting visual` (WebView2's window-to-visual hosting, which users can force), where focus stays in our process. Its positive control keeps the app in front and must exit 3.
  - **Findings for later items:**
    - Injected input arrives with device handle 0. SendInput can't test device attribution, and per-device findings should leave those events out.
    - Input queued before a focus change may still arrive after `WM_ACTIVATE`, and a key held across the change may never report its release. Capture needs its own gate, closed on `WM_ACTIVATEAPP`. The engine should treat keys still down at that point as interrupted, not stuck.
    - tao registers Raw Input for mice and keyboards on its hidden window when its event loop is created, inside `Builder::build`. Tauri's `Always` filter removes both registrations later in `Builder::build`, before any app window exists. Nothing else in tauri or wry registers.
    - tao calls `GetAsyncKeyState` for every key when its window gains focus, to replay keys that are already held. It stores nothing. Accepted (2026.09.27): it reads which keys are down at that moment, only when our window gains focus, and is not a keylogger. The dependency scan pins this read to tao's one call and to the one caller of the function that wraps it, the focus handler.
- **Dependency scan** (2026.09.27): the capture guard also scans the source of every crate the Windows build compiles for the banned capture APIs. It skips the crates.io `windows` and `windows-sys` bindings, which declare every API without calling it. Known calls are listed in `DEPENDENCY_ALLOWED` in `scripts/check-capture.mjs`, with their file and how many lines outside `use` declarations name the API. An upgrade that adds a line naming one of these APIs fails CI, even in a file that already has one. The only entries are tao's key-state read and its `Never`-filter input sink, which `Always` never reaches.
- **Browser keys** (2026.09.27): WebView2's browser shortcuts and default context menu are off in every build, set on the live webview in `src-tauri/src/browser_ui.rs`, because wry supports both settings but Tauri doesn't pass them on. `scripts/check-browser-keys.ps1` proves it against a debug build: the settings read back as off, F5 and Ctrl+R don't reload the page, and a right-click opens no menu. It passes in both hosting modes. Its positive control `BrowserKeys` starts the app with them left on (`KEYTRIAGE_BROWSER_KEYS`, debug builds only), must read them back as on, and must see F5, Ctrl+R and Browser Refresh start a navigation and a right-click open a menu. Ctrl+P rests on the same setting and isn't pressed, because a print dialog is hard to close. The settings don't cover everything a key test presses:
  - A Browser Refresh or Back key, or a mouse side button, arrives as `WM_APPCOMMAND`. The evidence conflicts: one probe that posted `WM_APPCOMMAND` saw a reload with the setting off, and the check's Browser Refresh key, sent through `SendInput` on runtime 153, starts no navigation. While a test runs, the navigation guard refuses any reload either way (see M1).
  - A lone Alt or F10 can put the window in menu mode, Alt+Space opens the window menu, and Alt+F4 closes the app.
  - Tab, Space and the movement keys still act on the page.
  - The OS keeps its own keys: the Windows key and its combinations, Ctrl+Alt+Del, Print Screen, the Copilot key, Sleep and Power, launch keys, and the accessibility shortcuts (Shift five times, right Shift or Num Lock held down).
- **Crash output** (2026.09.27): a crash dump can hold the ordered key events a process keeps in memory, so no dump may leave the machine or stay on disk. `src-tauri/src/crash_reports.rs` does three things:
  - **Crashpad never uploads WebView2 dumps.** The app creates the WebView2 environment itself, with custom crash reporting on, and hands it to the main window, which is built in code (`"create": false`). It repeats the options wry would set, so `additionalBrowserArgs` in `tauri.conf.json` has no effect. Chromium's own switches (`--disable-breakpad`, `--disable-crash-reporter`) were tested and don't stop uploads.
  - **WebView2 dumps are deleted.** Crashpad still writes a dump of about 7 MB, which can hold page memory, into `EBWebView\Crashpad\reports`. The app deletes it and its attachments at startup, when WebView2 reports a failed process, and at exit. Crashpad's other files keep IDs and the app's name, not memory.
  - **The app's own crashes skip Windows Error Reporting.** `SEM_NOGPFAULTERRORBOX` is set before anything else runs. That covers Rust aborts, which skip every in-process handler, and access violations. The cost is that keytriage crashes don't show in Reliability Monitor or the event log.

  `scripts/check-crash-reports.ps1` checks all three against a debug build without crashing anything: the browser runs with custom crash reporting, the app watches for failed processes, planted dumps go at startup and at exit, and the error mode skips Windows Error Reporting. Its positive control (`KEYTRIAGE_CRASH_REPORTS`, debug builds only) turns them off and must catch all five. A behavioral crash test was left out, because its positive control would upload a report. What stays outside the app's control:
  - a WebView2 dump sits on disk between the crash and the sweep: milliseconds for a failed process, until exit for a dump WebView2 doesn't report, and until the next start for a crash while WebView2 shuts down, which finishes after the app has exited;
  - a crash of WebView2's browser process that Crashpad doesn't catch, for example when its handler can't start, goes to Windows Error Reporting, because that process sets its own error mode. WER then writes a dump to `C:\ProgramData\Microsoft\Windows\WER\Temp` and sends the crash signature, and with Optional diagnostic data it can send the dump. The renderers already run without WER;
  - hang reports, dumps that someone takes on purpose (Task Manager, ProcDump, a debugger), an admin's WER LocalDumps setting, memory paged to `pagefile.sys` or `hiberfil.sys`, and a memory dump after a system crash;
  - the WebView2 runtime's own connections to Microsoft, which run whatever the app does. M5's privacy document names them.
- **Diagnostic engine** (2026.09.28, defaults for the owner to review): `crates/diagnostics` turns a test's events into findings with `diagnose()`. It is std only, with no floats, clocks or hash-order iteration, so the same test always gives the same report. `RULES` (1, and 2 with the guided test's `PROMPT_MERGE_US`) is bumped whenever a threshold, bin edge or rule changes, so M4 compares only like with like. The numbers come from switch datasheets, QMK, ZMK and laptop firmware debounce, published chatter tools and keystroke timing studies, and each one's reason sits beside it in `params.rs`.
  - **Chatter.** A second press of the same key counts as chatter when the release-to-press gap is under 20 ms, or when it is under 100 ms and either press lasted under 20 ms. Pairs under 36 ms are shown as borderline and never counted. Readings under 1 ms, or across two handles, aren't timed. A short phantom joins only the nearer of two real presses, and a key whose presses are nearly all short is taken as tapped by its firmware. Only presses in the key's own rounds count, and a finding needs 2 or more affected presses; one is a note, and free typing only gives notes. Confidence comes from the Wilson 95% lower bound of the rate and from the rounds that reproduce it: Very high needs 5 or more affected presses, at least 5%, in every round of 3 or more. Three or more chattering keys, or a keyboard that reports every 16 ms or slower, cap confidence at Medium; at 16 ms, a key with more near misses than hits gives no finding at all.
  - **Dead key.** A round that asked for 3 or more presses and got no key-down is silent, unless it was mostly paused, the key was already down, the key arrived from another keyboard, software injected input, or the keyboard was at its rollover limit for most of the round. High and Very high need other keys to answer their rounds, before and after for Very high, so the user was there. A round that got another key's code instead is noted, not counted.
  - **Stuck key.** A key-down with no release by the end of the test after 2 s or more, or a hold of 2 s or more through the key's own round. Other long holds, and keys down at a pause, are never stuck; a key interrupted at a pause may report its release or repeats after the resume.
  - **Saved data.** `Report::saved()` is the only data that may be written: per-key counts and histograms of holds, release gaps and press-to-press intervals, with no timestamps, orders, lists or key pairs. It keeps only prompted keys and keys a finding or note names, so free typing's keys stay out. Hold histograms are still biometric, which M5's privacy document says.
  - **For the owner to review:** the 20, 36 and 100 ms limits, which count a phantom of 20 ms or more after a gap of 20 ms or more only as borderline (laptop firmware with a 30 ms release lock can chatter there); prompted-only chatter; and the canned example in `README.md` and `src/app/app.ts`, which the engine would word differently ("a rate of at least 8.5%", "came 5 ms after a release", very high rather than high for 14 of 100 in 3 of 3 rounds).
- **Guided test and report** (2026.09.28). The owner's calls:
  - **Keys.** The test prompts every plain key of the chosen layout (digits, letters and punctuation: 47 on ANSI, 48 on ISO) in the drawing's reading order, each key once per round, 10 presses over 3 rounds. That is about 1 400 presses, 10 to 12 minutes. Modifiers, F-keys, Esc, Tab, Caps Lock, Enter, Backspace, Space, arrows, navigation and lock keys, Print Screen and Fn stay out, because they act on the OS or the page or send no code. The plan is data, so M4's swap retest reuses it.
  - **Board.** The Start screen's Board column (hot-swap, soldered, laptop) is built in M3, with Hot-swap preselected, so each finding's next steps fit the board. M4 keeps the swap test.
  - **Skip** keeps the round as asked. A skipped round with no key-down is silent, which is evidence of a dead key.

  Defaults for the owner to review:
  - **Counting.** Rust runs the test: a pure `Guide` in `crates/diagnostics` counts presses and stamps each round on the events' own clock, and the page only draws what it sends. A key-down counts toward the prompt when it is the prompted key, comes from the chosen keyboard (never handle 0) and lands at least 100 ms (`PROMPT_MERGE_US`) after that key's last release. Closer key-downs may be chatter, so they show on the drawing but never move the prompt on. `RULES` becomes 2 for this limit. A round closes at the release of its 10th counted press.
  - **Pause and skip.** The Pause button stops capture, as a focus loss does. Either pause drops the open round, and the resume repeats it from 0. Skip closes the round as asked and moves on. Skip while paused moves on without a round and continues the test. End test keeps an open round that counted a press and drops one that didn't, so ending early never makes a silent round.
  - **Keys on the page.** During a test the page cancels every key event without reading which key it was, and keeps focus off its buttons, which work by mouse only. OS keys such as Win, Alt+F4 and Print Screen still act, and at worst pause the test.
  - **Export.** The file holds `Report::saved()` and a fixed header: app, format, rules, bin edges and a one-line note. It holds no findings text, key names, layout, dates or keyboard IDs. Rust writes it through the Windows Save dialog. The page sends only a file name, which Rust checks against `keytriage-YYYY.MM.DD-HHMM.json`.
  - **Design departures:**
    - the prompt's count shows counted presses, so 10 / 10 always means the round is done, while the drawing counts every key-down;
    - the chatter card's histogram is the key's saved release-gap histogram in 6 bars (<4, 4–12, 12–20, 20–36, 36–100 and 100+ ms), and dead and stuck cards have none;
    - the timing limits sentence comes from the engine's polling estimate, not a fixed 1 ms;
    - the clean card lists the plan and the engine's notes, and its next test ends "then test again.";
    - the demo's "dropped presses" and "sticking key" findings don't exist, and "Run the swap test" waits for M4;
    - where the mockups and the demo differ, the demo wins, except for its stand-in findings. Where the specification and the drawn components differ, the components win.

## M0: Placeholder (as soon as possible)

- [x] Add `LICENSE` (Apache-2.0).
- [x] Scaffold Angular + Tauri v2, plus the Rust workspace with `crates/input` and `crates/diagnostics`.
- [x] Generate the app and installer icons from `docs/brand/mark.svg`. The source is `docs/brand/app-icon.svg`, with the mark on a paper-colored tile, because its dark jaws disappear on a dark taskbar.
- [x] A plain placeholder window that shows one canned finding as text: the key, the evidence, the causes and the next test. It gets no styling until the design lands.
- [x] Turn on the guardrails from the first commit:
  - [x] a Content Security Policy that blocks outbound requests from the page, a navigation guard that keeps the window on the app, and no HTTP or updater plugins;
  - [x] a CI check that fails on `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon` or `RTCPeerConnection` in the frontend, and on HTTP crates in the Rust workspace. `RTCPeerConnection` is on the list because the CSP does not cover WebRTC. The crate check runs `cargo tree --workspace` for the Windows target, because `Cargo.lock` also lists `reqwest` for mobile targets;
  - [x] a CI check that fails on `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx` or `WH_KEYBOARD_LL`, and on a Tauri `DeviceEventFilter` other than `Always`, because `Never` makes tao register with `RIDEV_INPUTSINK`. A planted `DeviceEventFilter::Never` is one of its positive controls;
  - [x] a CI check that fails on a direct dependency that is not on its crate's allowlist.

**Done when:** CI builds the app on Windows, the placeholder renders, and test PRs that add `fetch(` or `RIDEV_INPUTSINK` each fail their check.

## M1: Input

- [x] Spike first: receive `WM_INPUT` in the Tauri main window without an input sink, and confirm that input stops when the window loses focus. Record the approach here. Raw Input registration is per process and the last call wins, so nothing may call `set_device_event_filter` once capture has registered.
- [x] Before capture lands, turn off WebView2's browser shortcuts and default context menu (`SetAreBrowserAcceleratorKeysEnabled`, `SetAreDefaultContextMenusEnabled`) through `with_webview`. The key test presses F5, Ctrl+R and Ctrl+P, which reload or print the page. Tauri has no setting for this.
- [x] Decide how crash output stays local. WebView2 sends renderer crash reports to Microsoft by default (`IsCustomCrashReportingEnabled` is off), and a renderer dump can hold the live event stream.
- [x] Device list: name, VID/PID, device path, and manufacturer and product strings. `keytriage_input::keyboards()` lists every keyboard Raw Input knows, with its handle (the value each key event carries), path, VID and PID, the HID manufacturer and product strings, and Windows' container ID. The strings come through a handle opened with no access rights, which can't read the keyboard's reports. One keyboard can show up once per HID collection. The container groups an external keyboard's collections, but everything built into the computer shares one container, so events are attributed by handle. The debug echo prints the list, and the app will expose it when the keyboard picker (M3) needs it.
- [x] Live event view: key position, down or up, a high-resolution timestamp and the source device, held in memory only. The Rust side (2026.09.27, the owner's call): each event is timestamped when the app reads it, and a test session (`src-tauri/src/test_session.rs`) keeps the test's events in order in memory and sends each one to the page as a `test:event` event. The page starts and stops a test with the `start_test` and `stop_test` commands. Drawing the events waits for the design (M3).
- [x] Capture starts only with a test and stops when the test ends or the window loses focus. Capture exists only inside `start_test` and `stop_test`. When the app loses the foreground, capture closes an in-process gate, which drops input queued before the switch, unregisters Raw Input and reports a pause. When the app is back in front it registers again and reports a resume. Every activation message counts, because a window that Windows activated at launch without the foreground gets only `WM_NCACTIVATE` when the foreground arrives. The test pauses and resumes rather than ending (2026.09.27, the owner's call). A test started in the background begins with a pause, so every resume follows one. A pause lists the keys still down as interrupted, because their release may never arrive; a key still held at the resume reports its release after it. If the page's process dies, the test ends and the page reloads. `scripts/check-focus-capture.ps1` also checks that capture holds no registration while paused and registers again on return. Its positive control `Registration` (`KEYTRIAGE_KEEP_REGISTRATION`, debug builds only) keeps a registration while paused and must be caught.
- [x] While a test runs, refuse reloads and history moves. A Browser Refresh or Back key and a mouse side button reach WebView2 as `WM_APPCOMMAND`, which the browser keys setting doesn't cover. While a test runs, the navigation guard refuses every navigation, and the page survives a refused reload. The guard is tested on its own: in `scripts/check-browser-keys.ps1`, F16 makes the page reload itself, which no setting stops, and the guard must refuse it. The echo counts page loads as they start, because a refused navigation still finishes, and prints each refusal. The positive control `Reloads` (`KEYTRIAGE_RELOADS`, debug builds only) turns only the refusal off and must see the page reload. History moves inside one document (`pushState`) never reach the guard, so the test page must not add history entries.
- [x] Check device attribution with two physical keyboards. `SendInput` can't do this, because injected input has device handle 0. Start a debug build with `KEYTRIAGE_ECHO`, press keys on each keyboard during a test, and check that each key's `device=` is one of that keyboard's handles in the `kt-input: keyboard handle=` list. Done 2026.09.27 on the owner's machine, with a SONIX Galaxy80 over USB and the Razer Blade's own keyboard: 35 presses on the Galaxy80 and 27 on the Blade, some of them interleaved, each landed on that keyboard's handle, with a release for every press and none from an unlisted handle. The Galaxy80 reports Apple's vendor ID (`05ac`), as SONIX firmware often does for Mac compatibility, so identification (Later) can't trust the VID alone.

**Done when:** events from two connected keyboards are attributed to the right device, and an automated check proves that no events arrive after the window loses focus.

## M2: Diagnostic engine

- [x] A common result type: finding, confidence, evidence, possible causes and next test.
- [x] Detectors for chatter (repeated transitions within a threshold), dead keys (a prompted key produces nothing) and stuck keys (a down without an up). A key listed as interrupted at a pause is not stuck, and its release or repeats after the resume are expected.
- [x] Per-key aggregates (counts and interval histograms) as the only data that can be saved.
- [x] Fixtures are synthetic event streams, never recorded typing. Each detector has fault fixtures and clean fixtures. Clean fixtures must produce no finding, including fast deliberate double presses.

**Done when:** every fixture produces its expected findings, and the clean fixtures produce none.

## M3: Guided test and report

- [x] **Design first.** Stop and ask the owner for the UI design. Nothing below starts until it is in `docs/design/`. The owner's design arrived 2026.09.28 from Claude Design and covers M3 and M4:
  - `keytriage.dc.html` holds the static mockups (start, test running, paused, injected input, findings, clean) and the specification: tokens, type, spacing and component states;
  - `keytriage demo.dc.html` is the interactive demo, with the swap test;
  - `_ds/` holds the directive//01 tokens and components it builds on.

  The design runtime (`support.js`) is left out, because it loads React from a CDN. The mockups name Google Fonts and the design system suggests Lucide from a CDN. The app instead bundles the WOFF2 fonts, as the specification says, and uses no icon set. The demo's findings and export are stand-ins; the app shows the engine's findings and exports `Report::saved()`.
- [x] Pick a keyboard and a generic layout: ANSI or ISO, in full size, TKL, 75%, 65% or 60%. The Start screen lists keyboards by device, and any entry picks all of that device's entries.
- [x] Live event view on screen, drawn from the `test:event` events. The test screen lists the 60 newest events in memory and shows 16, draws the keys held on the tested keyboard, and marks focus losses, interrupted keys and injected input.
- [x] Guided key test: press a key N times over several rounds, with live counts. The page shows Rust's prompt, count and progress as sent, draws each key's tally, and offers Pause, Continue, Skip this key and End test.
- [x] Findings list, with the flagged keys shown on the keyboard. The plan's last view or End test calls `end_test`, and the findings screen numbers Rust's findings, tags each flagged key with its number and offers New test and Test again.
- [x] Export a report. A test fails if a report contains an ordered event sequence. Export report sends `export_report` only a file name from the test's start, and the footer says where the file was saved or why it wasn't.

**Done when:** a synthetic chatter stream shown through the UI produces the expected finding on the right key, and its exported report passes the no-sequence test.

## M4: Swap experiment

- [ ] Ask what kind of board it is: hot-swap, soldered, or laptop and other scissor-switch keyboards. Moved into M3 on the owner's call (2026.09.28): the Start screen's Board column.
- [ ] Hot-swap: guide the swap of the suspect switch with a known-good key, then retest both keys.
- [ ] Update the diagnosis: a fault that follows the switch points to the switch; a fault that stays points to the socket, PCB or matrix.
- [ ] Soldered and laptop boards get their own next steps, with no swap.

**Done when:** synthetic before-and-after streams for "fault follows the switch" and "fault stays" each produce the right updated diagnosis.

## M5: v0.1.0

- [ ] A release workflow for Windows: an NSIS installer on GitHub Releases, unsigned, with SHA-256 checksums.
  - Set `bundle.publisher` to the name a future code-signing certificate would carry, before the first installer ships. It defaults to "github" from the identifier, and the installer keys its registry entry on it, so changing it later loses the previous install location.
  - Decide `bundle.windows.webviewInstallMode`. The default downloads the WebView2 bootstrapper from Microsoft when WebView2 is missing.
  - Ship the third-party notices: Angular's `3rdpartylicenses.txt`, which sits outside `frontendDist`, and the Rust crates' licenses.
- [ ] README with install steps, a note on unsigned builds and screenshots of the live app.
- [ ] `CONTRIBUTING.md`: how to add a detector and its fixtures.
- [ ] `SECURITY.md`, plus a privacy contract document that lists each promise and how it is enforced. It also names what the WebView2 runtime fetches from Microsoft on its own, such as its variations seed, outside the app's code.
- [ ] Issue templates for "Wrong diagnosis" and "Missed fault". They ask for the exported report, never a recording of typing.
- [ ] Dogfooding log in `docs/dogfooding.md`.
- [ ] CI is green, error messages are understandable, and there are no known critical bugs.

**Done when:** the released installer runs on a clean Windows machine, and a real chattering or dead key is found and explained through the guided test.

## M6: Parts and prices (first in v0.2)

- [ ] Switch catalog: MX-style switches first (3-pin or 5-pin, mount type, sensing method), seeded from open switch data where the license allows. Every fact records its source.
- [ ] Compatibility rules that state their reasoning, for example "MX hot-swap socket, 5-pin fits a PCB-mount board". A switch is never compatible just because it looks like MX.
- [ ] The price workflow: a scheduled job reads public product feeds from a curated list of retailers and commits a price and stock file with an "as of" date. A retailer is added only after its robots.txt and terms have been checked, and that check is noted in the list.
- [ ] When the swap test points at a switch on a hot-swap MX board, show compatible switches with their price, stock and date. Links open in the browser.
- [ ] A test fails if a stored link carries a referral or affiliate parameter.

**Done when:** the price workflow has run on its schedule in CI, the app lists compatible switches with snapshot prices after a switch finding, and the no-network guard still passes.

## Later

- Keyboard identification from usb.ids, QMK and VIA definitions and the SonixQMK database, with the source recorded for every fact. It fills in the board type automatically.
- Rollover, ghosting and matrix-pattern analysis, worded carefully: a missing key can be a design limit, not a fault.
- A HID descriptor inspector, behind an Advanced view.
- Repeated test sessions, compared over time.
- Linux (evdev and hidraw) behind the same `crates/input` interface.
- A CLI on the same crates.
- An opt-in background monitor for intermittent faults. It stays visible while on and saves aggregates only.
- Low-profile, optical, Hall effect and laptop keyboards.
- Community submissions for the keyboard and switch data.

## Not planned

- Chatter filtering or suppression. Keyboard Chatter Blocker does this.
- Firmware flashing.
- Affiliate links, or live price lookups from inside the app.
- Claims about PCB electrical faults that software can't observe.
- Saving ordered key events, or capturing keys outside a test.
- Telemetry, accounts or anything hosted.
- macOS for now.

## How we'll know it works

Evidence comes from dogfooding (n=1, recorded in the log): real faults found, and whether the swap test confirmed them. After release it also comes from public signals: "wrong diagnosis" and "missed fault" issues, detector PRs and downloads.
