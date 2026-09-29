<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/lockup-dark.svg">
    <img alt="keytriage" src="docs/brand/lockup.svg" height="44">
  </picture>
</h1>

**Find out why your keyboard misbehaves, and what to try next.** keytriage tests your keyboard, shows the evidence, ranks the likely causes, and walks you through the test that tells them apart.

A key that double-types, drops presses or sticks could be the switch, the socket or solder joint, the PCB or the firmware. Keyboard testers show that something is wrong, but not what to do about it. So people replace the wrong part, or the whole keyboard.

> **Status:** planning. There is nothing to install yet. Windows comes first. See [ROADMAP.md](ROADMAP.md).

## What a finding looks like

This is the finding the app shows for a synthetic chatter stream on a hot-swap board.

```text
E   Possible chatter, very high confidence

Evidence
  6 of 30 presses sent an extra key-down (a rate of at least 9.5%)
  The extra key-downs came 5 ms after a release
  The extra presses lasted 5 ms
  Reproduced in 3 of 3 rounds
  None of the 2 other tested keys showed it
  The keyboard's own debounce hides contact bounce shorter than its setting, often 5 ms

Other possible causes
  1. Switch contacts
  2. Hot-swap socket
  3. Firmware debounce

Next test
  Swap the E switch with the G switch and test both keys again.
  If the fault moves to G, the switch is the likely cause.
  If it stays on E, look at the socket or the PCB.

  Blow out the E switch with the key held down, or work contact cleaner
  into it while pressing it many times. Then test E again.

  If the keyboard's firmware lets you, raise its debounce time to 10 ms,
  then 15 ms, and test again.
```

## What it does

- **Tests** for chatter, dead keys and stuck keys on the keyboard you pick, by physical key position.
- **Explains** each finding with its evidence, a confidence level and the other possible causes. It never just says "broken".
- **Isolates** the fault with a guided switch-swap test on hot-swap boards, and gives different next steps for soldered and laptop keyboards.
- **Later:** identifies your keyboard, lists compatible switches with prices, tests rollover and inspects HID.

## The privacy contract

- **Not a keylogger.** It reads keys only while a test runs and its window is focused. It has no background capture, and CI fails on the Windows APIs that would allow it.
- **No typing saved.** The order of keys stays in memory. Saved reports hold per-key counts and timings only, because an ordered list of keys is the text you typed.
- **No network.** There is no networking code, no telemetry and no auto-updater. Windows' WebView2 runtime, which draws the window, talks to Microsoft on its own; [PRIVACY.md](PRIVACY.md) lists what it does.
- **Open source**, so you can check all of this instead of trusting it.

[PRIVACY.md](PRIVACY.md) maps each promise to the code that keeps it and the check that proves it.

## Contributing

Each detector is a small Rust module with event-stream fixtures. [CONTRIBUTING.md](CONTRIBUTING.md) explains how to add one.

## License

[Apache-2.0](LICENSE)
