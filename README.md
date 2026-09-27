<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/lockup-dark.svg">
    <img alt="keytriage" src="docs/brand/lockup.svg" height="44">
  </picture>
</h1>

**Find out why your keyboard misbehaves, and what to try next.** keytriage tests your keyboard, shows the evidence, ranks the likely causes, and walks you through the test that tells them apart.

A key that double-types, drops presses or sticks could be the switch, the socket or solder joint, the PCB or the firmware. Keyboard testers show that something is wrong, but not what to do about it. So people replace the wrong part, or the whole keyboard.

> **Status:** planning. There is nothing to install yet. Windows comes first. See [ROADMAP.md](ROADMAP.md).

## What a finding will look like

The report below is planned. It is not real output yet.

```text
E   possible chatter, high confidence

Evidence
  14 of 100 presses sent an extra key-down 4 to 9 ms later
  reproduced in 3 of 3 rounds
  no neighbouring keys affected

Likely causes
  1. switch contacts
  2. hot-swap socket or solder joint
  3. firmware debounce

Next test
  Swap the E switch with the G switch and test both keys again.
  If the fault moves to G, the switch is the cause.
  If it stays on E, look at the socket or the PCB.
```

## What it does

- **Tests** for chatter, dead keys and stuck keys on the keyboard you pick, by physical key position.
- **Explains** each finding with its evidence, a confidence level and the other possible causes. It never just says "broken".
- **Isolates** the fault with a guided switch-swap test on hot-swap boards, and gives different next steps for soldered and laptop keyboards.
- **Later:** identifies your keyboard, lists compatible switches with prices, tests rollover and inspects HID.

## The privacy contract

- **Not a keylogger.** It reads keys only while a test runs and its window is focused. It has no background capture, and CI fails on the Windows APIs that would allow it.
- **No typing saved.** The order of keys stays in memory. Saved reports hold per-key counts and timings only, because an ordered list of keys is the text you typed.
- **No network.** There is no networking code, no telemetry and no auto-updater. Price data ships inside the app, and links open in your browser.
- **Open source**, so you can check all of this instead of trusting it.

## Contributing

Each detector is a small Rust module with event-stream fixtures. A contributor guide arrives with v0.1.

## License

[Apache-2.0](LICENSE)
