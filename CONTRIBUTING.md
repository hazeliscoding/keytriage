# Contributing

This guide covers setup, the checks a change must pass, and how to add a detector. [AGENTS.md](AGENTS.md) holds the full working rules, and [PRIVACY.md](PRIVACY.md) the promises every change keeps.

## Setup

- Windows 11 x64 with the WebView2 runtime, which Windows 11 includes.
- Rust stable with the MSVC toolchain, which needs Visual Studio's C++ build tools.
- Node 24, then `npm ci`.
- PowerShell 7 for the app checks.
- cargo-about 0.9.2, only to build the installer: `cargo install cargo-about --locked --version 0.9.2`.

`npm run tauri dev` runs the app. `npm start` serves the page alone in a browser, without Rust.

## Checks

CI runs these on every pull request, and all must pass:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm test -- --watch=false
npm run build; node scripts/check-network.mjs --bundle
node scripts/check-capture.mjs; node scripts/check-dependencies.mjs
node --test scripts/check-network.test.mjs scripts/check-capture.test.mjs scripts/check-dependencies.test.mjs scripts/check-notices.test.mjs scripts/check-release.test.mjs
```

A change to capture or webview code also needs the app checks in [AGENTS.md](AGENTS.md): the focus, browser keys and crash reports checks, each with its positive controls. The focus and browser keys checks take the foreground for about 15 seconds.

## Rules

- Never commit, paste or attach a recording of real typing. Fixtures are synthetic event streams built with `Synth` in `crates/diagnostics/src/fixture.rs`.
- `crates/diagnostics` is pure Rust with no OS, Tauri or I/O code, and the same events always give the same report.
- Findings describe evidence and likelihood. Never say a part is broken.
- Only `Report::saved()` is written to disk. Nothing may save the order of keys.
- No networking code. A new crate needs a check that it neither captures keys nor opens connections, and an entry in `ALLOWED` in `scripts/check-dependencies.mjs`.
- New UI waits for the owner's design in `docs/design/`.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org/), such as `feat(diagnostics): …`, one logical change each, with no AI attribution.

## Adding a detector

1. **Write the detector.** Add a module in `crates/diagnostics/src/` and call it from `diagnose()` in `lib.rs`, as `chatter`, `dead` and `stuck` are. It adds `Finding`s, each with the key, a confidence, its evidence, the possible causes and the next tests, and `Note`s for what falls short of a finding. A next test of `SwapSwitch` with a partner is what lets the swap test be offered.
2. **Name its kind and evidence** in `report.rs`: a `Kind` variant, an `Evidence` variant with its own evidence struct, and the arm in `Finding::kind`. Every number a finding cites is a field.
3. **Put its limits in `params.rs`**, each with the reason for its value beside it, and bump `RULES` in `lib.rs`. Every change to a threshold, bin edge or rule bumps it.
4. **Follow the compiler.** The new variants leave these matches incomplete:
   - `crates/diagnostics/src/words.rs`: the finding's title, evidence lines, causes and next steps, and the swap test's wording for the kind. Every sentence must pass `hedged()`.
   - `crates/diagnostics/src/swap.rs`: the swap offer's rate floor, and what clears the kind in the retest.
   - `crates/diagnostics/src/tests/words.rs`: `sample()`, a finding of each kind for the swap wording tests.
   - `src-tauri/src/view.rs`: the kind's name for the page, whether its card has a histogram, and `retest()`, which fakes the fault in the swap tests.
5. **Then what the compiler can't see:**
   - `src/app/ipc.ts`: add the kind's name to `type Kind`.
   - `crates/diagnostics/src/tests/words.rs`: a stream in `everything()` that gives the new finding, so `w02_every_rendered_string_is_hedged` checks its words, and the kind in the `kinds` lists of `every_swap_line()`, which `w11_every_swap_line_is_hedged` reads, and of `w12_a_side_with_another_finding_is_never_cleared_outright`.
   - `src-tauri/src/view.rs` tests: a run that gives the new finding in `res03_every_rendered_line_keeps_to_evidence_and_likelihood`, with its name in that test's list of kinds, and in the swap wording loop beside `dead_run()` and `stuck_run()` if the swap test can be offered for it.
   - `beyond_rounds()` in `crates/diagnostics/src/aggregate.rs`, if the evidence can lie outside the key's own rounds, as a stuck key's does.
   - `src-tauri/src/export.rs` names every saved field. A new saved number doesn't compile there until it has had a privacy review.
6. **Rewrite the goldens.** A `RULES` bump changes `src/app/testing/guided-chatter.json` and `swap.json`, and `golden_is_current` fails until they are rewritten. Run `cargo test -p keytriage golden` locally with `KEYTRIAGE_BLESS=1` set, and read the diff before committing. CI never rewrites them.

A finding's card is drawn from what Rust sends, so a new kind needs no page code beyond `ipc.ts`. Anything new on screen waits for the owner's design.

## Tests for a detector

- Put them in `crates/diagnostics/src/tests/<name>.rs`, and add `mod <name>;` to `tests/mod.rs`.
- Build each stream with `Synth` and diagnose it with `run()`. `only()` returns the one finding a stream must give, and `assert_clean()` checks that it gives none. The constants in `tests/mod.rs`, such as `E` and `G`, are scan codes.
- Each detector needs fault fixtures, which must give their finding with the expected evidence and confidence, and clean fixtures, which must give none. The clean fixtures include fast deliberate double presses and typing that only looks like the fault.

## Building the installer

```powershell
New-Item -ItemType Directory -Force target/notices
cargo about generate --locked --fail -c scripts/notices/about.toml -m src-tauri/Cargo.toml -o target/notices/THIRD-PARTY-RUST.txt scripts/notices/about.hbs
npm run tauri build -- --bundles nsis --config src-tauri/tauri.release.conf.json -- --locked
node scripts/check-notices.mjs
```

The installer lands in `target/release/bundle/nsis`. To rebuild a release, check out its tag first. The exe and the installer won't match the release byte for byte, because they carry build timestamps, so check a download against the release's `SHA256SUMS.txt` and its attestation instead. A local installer is never a release.
