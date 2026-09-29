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

## 2026.09.29: an induced dead key and the swap test

- **Build:** debug, commit `b07505c`, with the key picker.
- **Keyboard:** BY Tech Gaming Keyboard (`258A:0062`), USB, hot-swap, 75% ANSI.
- **What ran:** the C switch was pulled out of its socket with the keyboard unplugged, so C was dead by construction. Then Chosen keys with X and C, 3 rounds of 30 presses. C's rounds were skipped, because there was nothing to press. The findings screen offered the swap test, and the C switch went into X's socket and X's switch into C's.
- **What the app said:** a finding on C that offered the swap test. Its title and confidence were not recorded. The first report holds C with 0 key-downs in 3 silent rounds of 30 asked, and X with 108 key-downs and no extra key-down. After the swap: "Neither key showed the fault.", Low confidence, with "Both keys registered normally after the swap. Reseating the switches may have cleared a poor contact, or the fault comes and goes and didn't show in this test." The retest's report holds C with 93 and X with 100 key-downs, none extra.
- **Next:** nothing. The keyboard has all its switches back.
- **Ground truth:** induced. The fault was an empty socket, and seating a switch in it cleared it, which is what the outcome says.
- **Follow-up:** a natural fault is still to log. The confidence shown for an induced dead key should be recorded next time.

## 2026.09.29: the published installer on a clean account

- **Build:** release v0.1.0, from `9c46e1e`, as published on GitHub.
- **Machine:** the owner's PC, in a new standard Windows account.
- **What ran:** the installer and `SHA256SUMS.txt`, downloaded through a browser, the release notes' PowerShell hash check, then the installer and the app.
- **What Windows said:** the hash check printed True. SmartScreen showed "Windows protected your PC" and "Microsoft Defender SmartScreen prevented an unrecognized app from starting. Running this app might put your PC at risk.", with "App: keytriage_0.1.0_x64-setup.exe" and "Publisher: Unknown publisher" under More info, then Run anyway. The browser's own download prompt was not recorded.
- **What the app did:** it installed with no admin prompt and opened on its Start screen.
- **Ground truth:** the README's Unsigned builds section names the same prompt and the same way past it.
- **Follow-up:** none.

## Still to log

- A real replug during the swap test, which the unit tests cover only on a synthetic device path.
- A natural chattering or dead key found through the guided test, with the swap outcome and the ground truth. The intermittent C key on an Akko keyboard is a candidate.
