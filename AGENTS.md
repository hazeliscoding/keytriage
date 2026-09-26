# AGENTS.md

These are the working rules for agents in this repo. keytriage is a local-first desktop app (Rust + Tauri v2 + Angular, Windows first, Apache-2.0) that diagnoses keyboard faults from evidence and guides the tests that isolate them.

## Sources of truth

- `README.md`: the pitch and the privacy contract.
- `ROADMAP.md`: decisions already made, the milestones, and what is out of scope. Check it before proposing features. Respect those decisions unless the owner reopens them.
- Work from the next unchecked item in `ROADMAP.md`. Don't build past the current milestone without asking.

## The privacy contract (hard rules)

Software that reads a keyboard has to earn trust. Never break these rules, not even in dev tooling that ships.

- **Foreground capture only.** Register Raw Input on the app's own window with no input sink. Never use `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx` or `WH_KEYBOARD_LL`. Capture runs only during a test.
- **Never save the order of keys.** Ordered events stay in memory. Anything written to disk (reports, logs, settings, crash output) holds per-key aggregates only.
- **Keys are physical positions** (scancodes). Don't translate them to characters beyond the labels on the drawn keyboard.
- **No networking code.** No `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon`. No Tauri HTTP or updater plugins. No HTTP crates in the app. Price data is bundled, and links open in the browser.
- **No remote assets.** Fonts and icons are self-hosted.
- **No recorded typing anywhere.** Fixtures are synthetic event streams. Never commit, paste or attach a recording of real typing.

## Diagnostics

- The engine in `crates/diagnostics` is pure Rust. It has no OS, Tauri or I/O code.
- Every detector returns the common result: finding, confidence, evidence, possible causes and next test.
- Findings describe evidence and likelihood. Never say a part is broken. Say "possible chatter, high confidence" and list the other causes.
- Each detector has fault fixtures and clean fixtures. Clean fixtures, including fast deliberate double presses, must produce no finding.
- Diagnosis is deterministic. No LLM in the diagnosis path.

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
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` lint the Rust code.
- `npm run tauri build -- --no-bundle` builds the release binary without an installer.

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
