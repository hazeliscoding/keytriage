# Roadmap

keytriage is a local-first desktop app (Rust + Tauri v2 + Angular, Windows first) that diagnoses keyboard faults from evidence and guides the tests that isolate them. This file tracks what gets built, in what order, and the decisions already made.

## Decisions (2026-09-25)

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
  - A CI check fails on `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx` and `WH_KEYBOARD_LL`, with a positive control.
- **Only aggregates are saved.** The ordered event stream lives in memory for the live view and the running test. Reports and anything else written to disk hold per-key counts and timing statistics only, because an ordered list of keys is the text that was typed.
- **No network in the app.**
  - There is no networking code and no auto-updater.
  - The Content Security Policy blocks outbound connections.
  - A CI check fails on web request APIs and HTTP crates, with a positive control.
  - Data such as prices ships inside the app, and links open in the browser.
- **Prices come from a CI snapshot.** A scheduled GitHub workflow reads public retailer product feeds, such as Shopify's `/products.json`. It respects robots.txt and each shop's terms, and commits a price and stock file with an "as of" date. The app bundles that file. Prices refresh with releases.
- **No affiliate links.** Parts links are plain and do not earn money.
- **Deterministic diagnosis.** Measurements go through rules to findings. No LLM in the diagnosis path.
- **Honest limits.** Software sees what the firmware reports after its own debounce, and timing resolution depends on the keyboard's polling rate. Reports say so where it matters.
- **The UI design comes from the owner.** Beyond the M0 placeholder, no UI code (templates, styles or layout) until that design is in `docs/design/`.
- **License:** Apache-2.0. The repo is public.
- **Brand** is option B, "Focus": viewfinder corners around a single key, for isolating the fault. The wordmark is Schibsted Grotesk Bold, lowercase, with -0.02em letter spacing, converted to vector paths. Ink is `#17161a` with a signal-orange accent `#c2410c`. The dark variants use `#ecebe6` and `#f59e62`. The assets are in `docs/brand/`.

## M0: Placeholder (as soon as possible)

- [x] Add `LICENSE` (Apache-2.0).
- [ ] Scaffold Angular + Tauri v2, plus the Rust workspace with `crates/input` and `crates/diagnostics`.
- [ ] Generate the app and installer icons from `docs/brand/mark.svg`.
- [ ] A plain placeholder window that shows one canned finding as text: the key, the evidence, the causes and the next test. It gets no styling until the design lands.
- [ ] Turn on the guardrails from the first commit:
  - [ ] a Content Security Policy that blocks outbound connections, and no HTTP or updater plugins;
  - [ ] a CI check that fails on `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon` in the frontend, and on HTTP crates in the Rust workspace;
  - [ ] a CI check that fails on `RIDEV_INPUTSINK`, `RIDEV_EXINPUTSINK`, `SetWindowsHookEx` or `WH_KEYBOARD_LL`.

**Done when:** CI builds the app on Windows, the placeholder renders, and test PRs that add `fetch(` or `RIDEV_INPUTSINK` each fail their check.

## M1: Input

- [ ] Spike first: receive `WM_INPUT` in the Tauri main window without an input sink, and confirm that input stops when the window loses focus. Record the approach here.
- [ ] Device list: name, VID/PID, device path, and manufacturer and product strings.
- [ ] Live event view: key position, down or up, a high-resolution timestamp and the source device, held in memory only.
- [ ] Capture starts only with a test and stops when the test ends or the window loses focus.

**Done when:** events from two connected keyboards are attributed to the right device, and an automated check proves that no events arrive after the window loses focus.

## M2: Diagnostic engine

- [ ] A common result type: finding, confidence, evidence, possible causes and next test.
- [ ] Detectors for chatter (repeated transitions within a threshold), dead keys (a prompted key produces nothing) and stuck keys (a down without an up).
- [ ] Per-key aggregates (counts and interval histograms) as the only data that can be saved.
- [ ] Fixtures are synthetic event streams, never recorded typing. Each detector has fault fixtures and clean fixtures. Clean fixtures must produce no finding, including fast deliberate double presses.

**Done when:** every fixture produces its expected findings, and the clean fixtures produce none.

## M3: Guided test and report

- [ ] **Design first.** Stop and ask the owner for the UI design. Nothing below starts until it is in `docs/design/`.
- [ ] Pick a keyboard and a generic layout: ANSI or ISO, in full size, TKL, 75%, 65% or 60%.
- [ ] Guided key test: press a key N times over several rounds, with live counts.
- [ ] Findings list, with the flagged keys shown on the keyboard.
- [ ] Export a report. A test fails if a report contains an ordered event sequence.

**Done when:** a synthetic chatter stream shown through the UI produces the expected finding on the right key, and its exported report passes the no-sequence test.

## M4: Swap experiment

- [ ] Ask what kind of board it is: hot-swap, soldered, or laptop and other scissor-switch keyboards.
- [ ] Hot-swap: guide the swap of the suspect switch with a known-good key, then retest both keys.
- [ ] Update the diagnosis: a fault that follows the switch points to the switch; a fault that stays points to the socket, PCB or matrix.
- [ ] Soldered and laptop boards get their own next steps, with no swap.

**Done when:** synthetic before-and-after streams for "fault follows the switch" and "fault stays" each produce the right updated diagnosis.

## M5: v0.1.0

- [ ] A release workflow for Windows: an NSIS installer on GitHub Releases, unsigned, with SHA-256 checksums.
- [ ] README with install steps, a note on unsigned builds and screenshots of the live app.
- [ ] `CONTRIBUTING.md`: how to add a detector and its fixtures.
- [ ] `SECURITY.md`, plus a privacy contract document that lists each promise and how it is enforced.
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
