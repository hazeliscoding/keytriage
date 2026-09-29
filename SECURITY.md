# Security

## Supported versions

Only the latest release gets fixes.

## Report a vulnerability

Report it privately through the Security tab's [Report a vulnerability](https://github.com/hazeliscoding/keytriage/security/advisories/new), never in a public issue. There is no email address for reports.

Never attach a recording of typing. An exported report, the steps that show the problem, or a synthetic example is enough.

## In scope

Anything that breaks a promise in [PRIVACY.md](PRIVACY.md), such as:

- keys read outside a focused test;
- the order of keys written anywhere: a report, a log, settings or crash output;
- a network connection made by `keytriage.exe`;
- a crash dump that leaves the machine;
- a way around the Content Security Policy or the navigation guard;
- the installer doing anything `PRIVACY.md` doesn't list. It carries Microsoft's WebView2 bootstrapper, which runs only when WebView2 is missing and then downloads the runtime from Microsoft.

## Out of scope

- The WebView2 runtime's own traffic to Microsoft, which `PRIVACY.md` describes, unless keytriage turns something on.
- SmartScreen's warning about the installer. The installer isn't code-signed, so the warning is expected.
- Limits that `PRIVACY.md` already lists, such as Windows' own keys acting during a test.

## What to expect

- keytriage has one maintainer, who aims to reply within a week.
- A fix ships in a release, and the advisory is published with it. You are credited if you want to be.
- To check that an installer is the one this repository released, see [Check it yourself](PRIVACY.md#8-check-it-yourself).
