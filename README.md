<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/lockup-dark.svg">
    <img alt="keytriage" src="docs/brand/lockup.svg" height="44">
  </picture>
</h1>

**Find out why your keyboard misbehaves, and what to try next.** keytriage tests your keyboard, shows the evidence, ranks the likely causes, and walks you through the test that tells them apart.

A key that double-types, drops presses or sticks could be the switch, the socket or solder joint, the PCB or the firmware. Keyboard testers show that something is wrong, but not what to do about it. So people replace the wrong part, or the whole keyboard.

> **Status:** v0.1.0 for Windows 11 (x64). Windows 10 should work but is untested. See [Install](#install) and [ROADMAP.md](ROADMAP.md).

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

These are from a real run on 2026.09.29, a hot-swap keyboard with X and C chosen.

![The Start screen with X and C chosen on the drawn keyboard](docs/screenshots/start.png)

![The guided test prompting C, 21 of 30 presses, with the live event list](docs/screenshots/test.png)

![A clean result for X and C, with the evidence and what it doesn't rule out](docs/screenshots/findings.png)

![The swap test's result after an empty socket was refilled: neither key showed the fault](docs/screenshots/swap-result.png)

## Install

1. Download `keytriage_0.1.0_x64-setup.exe` and `SHA256SUMS.txt` from the [latest release](https://github.com/hazeliscoding/keytriage/releases/latest) into one folder.
2. Check the installer before you run it. In PowerShell, in that folder, this prints `True`:

   ```powershell
   (Get-FileHash .\keytriage_0.1.0_x64-setup.exe).Hash -eq (Get-Content .\SHA256SUMS.txt).Split(' ')[0]
   ```

   `certutil -hashfile keytriage_0.1.0_x64-setup.exe SHA256` prints the hash to compare with `SHA256SUMS.txt`, and `sha256sum -c SHA256SUMS.txt` checks it in Git Bash. The release page shows each file's SHA-256 too. With the GitHub CLI, you can also check that this repository's release workflow built the file:

   ```powershell
   gh attestation verify .\keytriage_0.1.0_x64-setup.exe --repo hazeliscoding/keytriage
   ```

3. Run the installer. It installs for your user only, into `%LOCALAPPDATA%\keytriage`, with no admin prompt. Windows 11 includes WebView2, which draws the window. Where it is missing, the installer runs Microsoft's WebView2 bootstrapper, which downloads the runtime from Microsoft.

### Unsigned builds

The installer isn't code-signed, so Windows can't tell who published it.

- Your browser may warn that the file isn't commonly downloaded, and SmartScreen may show "Windows protected your PC". After checking the hash, choose **More info**, then **Run anyway**.
- Smart App Control, where it is on, blocks unsigned apps it doesn't recognize, and it has no exception for a single app. A build from source is unsigned too, so it doesn't get around this. The setting is in Windows Security, under App & browser control.

### Uninstall

Open Settings, then Apps, then Installed apps, and uninstall keytriage. Tick **Delete the application data** to also remove the app's WebView2 folder (`%LOCALAPPDATA%\io.github.hazeliscoding.keytriage`) and the registry key that remembers the install folder. Exported reports stay where you saved them. [PRIVACY.md](PRIVACY.md#6-what-keytriage-leaves-on-your-computer) lists everything the app leaves on your computer.

## The privacy contract

- **Not a keylogger.** It reads keys only while a test runs and its window is focused. It has no background capture, and CI fails on the Windows APIs that would allow it.
- **No typing saved.** The order of keys stays in memory. Saved reports hold per-key counts and timings only, because an ordered list of keys is the text you typed.
- **No network.** There is no networking code, no telemetry and no auto-updater. Windows' WebView2 runtime, which draws the window, talks to Microsoft on its own; [PRIVACY.md](PRIVACY.md) lists what it does.
- **Open source**, so you can check all of this instead of trusting it.

[PRIVACY.md](PRIVACY.md) maps each promise to the code that keeps it and the check that proves it. Report a broken promise privately, as [SECURITY.md](SECURITY.md) explains.

## Contributing

Each detector is a small Rust module with event-stream fixtures. [CONTRIBUTING.md](CONTRIBUTING.md) explains how to add one, and how to build the installer from source.

## License

[Apache-2.0](LICENSE). The installer puts the third-party notices in the install folder's `licenses` folder.
