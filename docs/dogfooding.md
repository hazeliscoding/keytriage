# Dogfooding log

This log is the n=1 evidence behind "How we'll know it works" in [ROADMAP.md](../ROADMAP.md). It has one entry per run on real hardware.

- Quote a finding as the app showed it.
- Never paste typed text, and never attach a report file. Say what the report holds instead.
- Write "not recorded" for anything that wasn't noted at the time.

Each entry gives the build (version or commit, debug or release), the keyboard (model, connection, board type and layout), what ran, what the app said, what happened next (a swap outcome or a repair), the ground truth, and follow-ups.

## 2026.09.27: device attribution with the debug echo

- **Build:** debug, with `KEYTRIAGE_ECHO`. Commit not recorded.
- **Keyboards:** a SONIX Galaxy80 over USB, and the Razer Blade's own keyboard. Board types and layouts not recorded.
- **What ran:** a test started by the echo, with presses on both keyboards, some of them interleaved.
- **What the app said:** 35 presses on the Galaxy80 and 27 on the Blade, each on one of that keyboard's handles in the `kt-input: keyboard handle=` list, with a release for every press and none from an unlisted handle.
- **Next:** nothing.
- **Ground truth:** every press landed on the keyboard it came from.
- **Follow-up:** the Galaxy80 reports Apple's vendor ID (`05ac`), as SONIX firmware often does, so keyboard identification can't trust the VID alone.

## 2026.09.28: export and the Save dialog

- **Build:** debug. Commit not recorded.
- **Keyboard:** not recorded.
- **What ran:** a guided test, then Export report.
- **What the app said:** the Save dialog opened, owned by the app's window.
- **Next:** nothing.
- **Ground truth:** the saved file holds only the header, the limits and per-key counts.
- **Follow-up:** none.

## 2026.09.28: the first real guided test

- **Build:** debug, from before the swap test, as the report's rules version 3 shows. Commit not recorded.
- **Keyboard:** not recorded.
- **What ran:** the guided test, ended after one round of 10 presses on each of 13 keys.
- **What the app said:** no finding. The card's wording was not recorded. The exported report holds the 13 prompted keys, with no pause, no injected input, no other keyboard and no 8 or 16 ms reporting schedule.
- **Next:** nothing.
- **Ground truth:** not recorded.
- **Follow-up:** none.

## Still to log

- A real replug during the swap test, which the unit tests cover only on a synthetic device path.
- The clean-machine install: the published installer, downloaded through a browser into a new standard Windows account, with its SHA-256 and SmartScreen's wording.
- A real chattering or dead key found through the guided test, with the swap outcome and the ground truth. The intermittent C key on an Akko keyboard is next.
