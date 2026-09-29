# AGENTS.md

These are the working rules for agents in this repo. keytriage is a local-first desktop app (Rust + Tauri v2 + Angular, Windows first, Apache-2.0) that diagnoses keyboard faults from evidence and guides the tests that isolate them.

## Sources of truth

- `README.md`: the pitch and the privacy contract.
- `PRIVACY.md`: each privacy promise, the code that keeps it and the check that proves it. When you rename, move or remove anything it names, update it in the same change.
- `ROADMAP.md`: decisions already made, the milestones, and what is out of scope. Check it before proposing features. Respect those decisions unless the owner reopens them.
- Work from the next unchecked item in `ROADMAP.md`. Don't build past the current milestone without asking.

## The privacy contract (hard rules)

Software that reads a keyboard has to earn trust. Never break these rules, not even in dev tooling that ships.

- **Foreground capture only.** Register Raw Input on the app's own window with no input sink. Never use `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx`, `WH_KEYBOARD_LL`, `GetAsyncKeyState`, `RegisterHotKey` or DirectInput in background mode, or a crate that wraps them. Build Raw Input flags only from the named foreground `RIDEV_` flags. Keep Tauri's `DeviceEventFilter` at `Always`: tao otherwise registers Raw Input for every keyboard at startup, and `Never` adds `RIDEV_INPUTSINK`. Capture runs only during a test.
- **Never save the order of keys.** Ordered events stay in memory. Anything written to disk (reports, logs, settings, crash output) holds per-key aggregates only.
- **The page writes nothing.** It never logs, stores or adds history entries, and its live rows stay in memory. During a test it cancels DOM key events without reading which key they carried. Rust writes the export from `Report::saved()`, and the page sends only the suggested file name.
- **Keys are physical positions** (scancodes). Don't translate them to characters beyond the labels on the drawn keyboard.
- **No networking code.** No `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon` or `RTCPeerConnection`. No Tauri HTTP, updater or shell plugins. No HTTP crates, sockets or `windows` networking features in the app. When prices arrive (M6), their data will be bundled and links will open in the browser. v0.1 has neither. The navigation guard in `src-tauri/src/lib.rs` keeps the window on the app. Don't widen it.
- **No remote assets.** Fonts and icons are self-hosted.
- **No recorded typing anywhere.** Fixtures are synthetic event streams. Never commit, paste or attach a recording of real typing.

## Diagnostics

- The engine in `crates/diagnostics` is pure Rust. It has no OS, Tauri or I/O code.
- Every detector returns the common result: finding, confidence, evidence, possible causes and next test.
- Findings describe evidence and likelihood. Never say a part is broken. Say "possible chatter, high confidence" and list the other causes.
- Each detector has fault fixtures and clean fixtures. Clean fixtures, including fast deliberate double presses, must produce no finding.
- Diagnosis is deterministic. No LLM in the diagnosis path.
- Bump `RULES` in `crates/diagnostics/src/lib.rs` whenever a threshold, bin edge or rule changes, and keep each threshold's reason beside it in `params.rs`.
- Rust runs the guided test. The `Guide` counts presses and stamps each round in the capture callback, and the page only draws what Rust sends. The page never sends a timestamp or a round boundary.
- Types that hold timestamps or ordered events (`Entry`, `HeldKey`, `Session`, `Core`, `Guide`) derive `Debug` at most in tests, because a printed list of them is the typed text. Only `Report::saved()` may be written anywhere.
- Fixtures come from the `Synth` builder in `crates/diagnostics/src/fixture.rs`, which later milestones' tests reach through the `fixtures` feature.

## Parts and prices

- Every fact in the keyboard and switch data records its source.
- Compatibility needs a stated reason. Never treat a switch as compatible just because it looks like MX.
- Prices come only from the scheduled CI snapshot of public product feeds, never from the app. Check a retailer's robots.txt and terms before adding it, and note that in the retailer list.
- No affiliate or referral parameters in links.

## UI design gate

The owner brings the UI design. Beyond the M0 placeholder, don't write UI (templates, styles, layout or components) until that design is in `docs/design/`. When a task reaches UI work, stop and ask the owner for the design. Never fill the gap with a generic look.

## Brand

- The assets are in `docs/brand/`. `-dark` files are for dark backgrounds.
- The wordmark is `KEY//TRIAGE` in Barlow Semi Condensed SemiBold, uppercase, with the slashes in the accent color, converted to vector paths. Use the SVGs, and don't re-typeset the wordmark with a web font.
- App icons put the mark on a paper-colored tile (`#f2ede2`), because the dark jaws disappear on a dark taskbar. The source is `docs/brand/app-icon.svg`. Regenerate the icons with `npx tauri icon docs/brand/app-icon.svg -o src-tauri/icons`, then delete the `android` and `ios` folders it adds.

## Commands

- `npm start` serves the UI in a browser. `npm run tauri dev` runs the desktop app.
- `npm test -- --watch=false` runs the UI unit tests. `cargo test --workspace` runs the Rust tests.
- `src/app/testing/guided-chatter.json` is written by `src-tauri/src/golden.rs`, and the UI tests replay it. After a deliberate change to the engine's words, a payload or the Guide, run `KEYTRIAGE_BLESS=1 cargo test -p keytriage golden` and read the file's diff before committing it, because a careless rewrite hides a changed finding.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` lint the Rust code.
- `npm run tauri build -- --no-bundle` builds the release binary without an installer.
- `node --test scripts/check-network.test.mjs scripts/check-capture.test.mjs scripts/check-dependencies.test.mjs scripts/check-notices.test.mjs scripts/check-release.test.mjs` runs the positive controls of the guards and the notices and release checks. `node scripts/check-network.mjs --bundle` (after `npm run build`), `node scripts/check-capture.mjs` and `node scripts/check-dependencies.mjs` run the guards.
- The Rust crates' notices come from cargo-about 0.9.2 (`cargo install cargo-about --locked --version 0.9.2`). Create `target/notices`, then run `cargo about generate --locked --fail -c scripts/notices/about.toml -m src-tauri/Cargo.toml -o target/notices/THIRD-PARTY-RUST.txt scripts/notices/about.hbs`. Use `--locked`, not `--frozen` or `--offline`, because the clarifications in `about.toml` read license files from GitHub. After that and `npm run build`, `node scripts/check-notices.mjs` checks that every linked crate, font and fixed section has its full notice. When a crate gets placeholder text, add a clarification that names the file holding its copyright line, with its SHA-256.
- The installer's resources, the three license files, live only in `src-tauri/tauri.release.conf.json`, never in `tauri.conf.json`. tauri-build copies resources on every cargo build and fails on a missing file, and the notices are generated. After the generate command, `npm run tauri build -- --bundles nsis --config src-tauri/tauri.release.conf.json -- --locked` builds the installer into `target/release/bundle/nsis`. CI builds it on every push, then zips the portable build: a `keytriage` folder with the same `keytriage.exe`, the marker `src-tauri/portable/keytriage.portable` and the three license files. `node scripts/check-notices.mjs --portable <folder>` checks that folder's license files against the overlay. CI uploads the zip, the installer and `SHA256SUMS.txt` as the `release` artifact. A local installer or zip is never a release.
- A version bump changes `package.json`, `package-lock.json`, `Cargo.toml` and `Cargo.lock` together, and `node scripts/check-release.mjs` fails until they agree. With `--tag vX.Y.Z` it also checks the tag, and with `--artifact <folder>` the staged release files: the zip and the installer for the version, and a `SHA256SUMS.txt` that lists them in that order. It keeps the publisher and the WebView2 mode set in `tauri.conf.json`, and the release overlay limited to files under `licenses/`.
- `pwsh scripts/check-installer.ps1 -Installer <setup.exe>` installs silently, checks the files, the Uninstall entry and the exe's version info, finds the installed app's rendered Begin test button through UI Automation, checks that its data folder holds only `EBWebView` and that a missing WebView2 runtime gets its dialog, then uninstalls and checks that nothing is left. It installs software, so it runs in CI, or with `-AllowLocal` in a throwaway VM, never on the owner's machine. `-PositiveControl Leftovers` and `DataFile` must make it exit 3.
- `pwsh scripts/check-connections.ps1`, from an elevated pwsh, proves that the release build opens no connection. For the run it turns on Windows Filtering Platform auditing and the DNS-Client log, then restores both. It fails on any connection or bind by `keytriage.exe`, and lists what WebView2's processes contacted, measured and never asserted. CI runs it on every push. `-PositiveControl` must make it exit 3. If it ever catches `keytriage.exe`, even through Windows code in the process, stop and ask the owner. Never allowlist a connection.
- `.github/workflows/release.yml` reruns all of CI on a `vX.Y.Z` tag on a commit on main, attests the zip, the installer and `SHA256SUMS.txt`, and drafts a GitHub release whose notes lead with the zip. `gh workflow run release.yml --ref main` is a dry run that prints the notes and creates nothing. Tagging and publishing are the owner's. Agents never tag, publish or edit a release.
- `pwsh scripts/check-focus-capture.ps1` proves that capture stops outside the foreground, `pwsh scripts/check-browser-keys.ps1` proves that WebView2's shortcuts and context menu do nothing, and `pwsh scripts/check-crash-reports.ps1` proves that WebView2 runs with custom crash reporting, that its dumps are swept, and that the app's crashes skip Windows Error Reporting. Build first with `npm run tauri build -- --debug --no-bundle`. Run the first two with and without `-Hosting visual`, and the crash reports check with and without `-Portable`, which runs a copy of the exe beside the portable marker. Every positive control must make its check exit 3: `-PositiveControl Background`, `Registration` and `UserPause` for the focus check, `BrowserKeys` and `Reloads` for the browser keys check, and `-PositiveControl` for the crash reports check. The first two take the foreground for about 15 seconds each, so run them when capture or webview code changes, or when the owner asks. They share `scripts/app-harness.ps1` and read the debug echo in `src-tauri/src/echo.rs`, which prints a scan code only for the marker keys F13 to F15. Keep every other key hidden. In the echo, F16 makes the page reload itself, and F17 makes it pause the test and continue it 3 s later.
- `src/app/testing/guided-chatter.json` pins every payload of the synthetic chatter run, and `golden_is_current` fails while it is out of date. After a deliberate change to the engine's words, a payload or the `Guide`, read the diff, then run `cargo test -p keytriage golden` with `KEYTRIAGE_BLESS=1` to rewrite it. The file is synthetic.
- Before adding a crate, check that it neither captures keys outside a focused test nor opens network connections, then add it to `ALLOWED` in `scripts/check-dependencies.mjs` in the same change.
- When the capture guard flags a banned API in a dependency, read the call. List it in `DEPENDENCY_ALLOWED` in `scripts/check-capture.mjs`, with a comment saying why, only if it can't read keys outside our focused window. Ask the owner before listing anything new, and record the decision in `ROADMAP.md`.
- The guards match names. Keep `fetch`, `WebSocket` and `EventSource` out of your own names and UI text, put comments that name a banned API on their own line, and keep `.device_event_filter(tauri::DeviceEventFilter::Always)` and `.plugin(navigation_guard())` in `src-tauri/src/lib.rs` in that form.
- The main window is built in `setup` with the WebView2 environment from `src-tauri/src/crash_reports.rs`, so keep `"create": false` on it, and put browser arguments in that environment, not in `tauri.conf.json`.
- With the marker `keytriage.portable` beside the exe, `run()` points every Tauri app folder at `keytriage-data` beside it (`src-tauri/src/portable.rs`), after proving that folder can be written. Resolve data folders through `app.path()`, never from `%LOCALAPPDATA%` directly, so the portable build keeps everything in its own folder.
- Unit tests in `src-tauri` must not reach Tauri's runtime. The test binary lacks the app's Common Controls manifest, so on Windows it exits with `0xc0000139`. Keep logic that needs tests in `crates/*`, or in plain functions like `is_app_url`.

## Working style

- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `ci:`, `build:`, `refactor:`). Keep each commit atomic, and use a scope when it adds clarity (`feat(diagnostics): …`).
- **No AI attribution** in commits or PRs. That means no `Co-Authored-By` trailers, no "Generated with" lines and no session links.
- **Checks:** automate acceptance checks instead of handing manual steps to the owner. Give every check that tests for an absence a positive control. For example, the capture guard must fail on a deliberate `RIDEV_INPUTSINK`.
- **Validation:** evidence comes from dogfooding (the log) and public async signals (issues, PRs, downloads). Don't plan interviews, recruiting or outreach.
- **Docs:** short and concise. Prefer editing `ROADMAP.md` over creating new planning documents.
- **Code comments:** explain why, not what. Only comment on what the code can't say for itself: a non-obvious constraint, a workaround and its cause, a Win32 quirk, or a line that keeps the privacy contract. Don't restate names or types, don't add boilerplate doc comments, and don't leave commented-out code or change notes.
- **Voice:** calm, short and declarative. No exclamation marks, no emoji.
- Dates are written `2026.09.24` and times use the 24-hour clock.
- Local Playwright output goes to `.playwright-mcp/`, which git ignores.
