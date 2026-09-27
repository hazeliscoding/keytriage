// Positive controls for the capture guard: each banned thing must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { hasRequiredFilter, scanCargoTree, scanSource } from './check-capture.mjs';

const ALWAYS = '.device_event_filter(tauri::DeviceEventFilter::Always)';
const LIB = `pub fn run() {\n    tauri::Builder::default()\n        ${ALWAYS}\n        .run(ctx);\n}\n`;
const script = join(dirname(fileURLToPath(import.meta.url)), 'check-capture.mjs');

// The flag rule applies to files that register Raw Input.
const rawInput = (body) => `let rid = RAWINPUTDEVICE {\n${body}\n};\nRegisterRawInputDevices(&[rid], size);\n`;

test('flags each background capture API', () => {
  const samples = [
    'dwFlags: RIDEV_INPUTSINK,',
    'dwFlags: RIDEV_DEVNOTIFY | RIDEV_EXINPUTSINK,',
    'SetWindowsHookExW(WH_MOUSE_LL, proc, h, 0)',
    'let id = WH_KEYBOARD_LL;',
    'let id = WH_KEYBOARD;',
    'unsafe { GetAsyncKeyState(0x41) }',
    'RegisterHotKey(hwnd, 1, MOD_ALT, 0x41)',
    'DirectInput8Create(h, DIRECTINPUT_VERSION, &IID, &mut di, None)',
    'dev.SetCooperativeLevel(hwnd, DISCL_BACKGROUND | DISCL_NONEXCLUSIVE)',
    '.device_event_filter(tauri::DeviceEventFilter::Never)',
    'app.set_device_event_filter(filter);',
    '.device_event_filter(Default::default())',
    'use tauri::DeviceEventFilter::{Never as Off};',
    'let f = RIDEV_INPUTSINK; // hidden after code',
  ];
  for (const s of samples) assert.ok(scanSource(s).length >= 1, s);
});

test('skips whole-line comments that explain the contract', () => {
  assert.deepEqual(scanSource('    // Never RIDEV_INPUTSINK: capture stays in the foreground.'), []);
  assert.deepEqual(scanSource('/// DeviceEventFilter::Never would add an input sink.'), []);
});

test('flags Raw Input flags that are not named foreground flags', () => {
  const bad = [
    '    dwFlags: RAWINPUTDEVICE_FLAGS(0x100),',
    '    dwFlags: SINK,',
    '    dwFlags: 1u32 << 8,',
    '    dwFlags: 0x80 + 0x80,',
    '    dwFlags: RIDEV_DEVNOTIFY\n        | EXTRA,',
  ];
  for (const body of bad) assert.equal(scanSource(rawInput(body)).length, 1, body);
  assert.equal(scanSource(`const SINK: u32 = 0x100;\n${rawInput('    usUsage: 6,')}rid.dwFlags = SINK;\n`).length, 1);
  assert.equal(scanSource(`${rawInput('    usUsage: 6,')}rid.dwFlags.0 |= 0x100;\n`).length, 1);
});

test('allows foreground Raw Input registration', () => {
  const ok = [
    '    usUsagePage: 0x01,\n    usUsage: 0x06,\n    dwFlags: RIDEV_DEVNOTIFY,\n    hwndTarget: hwnd,',
    '    dwFlags: RIDEV_DEVNOTIFY | RIDEV_NOLEGACY,',
    '    dwFlags: windows::Win32::UI::Input::RIDEV_DEVNOTIFY,',
    '    dwFlags: RAWINPUTDEVICE_FLAGS(0),',
    '    dwFlags: 0,',
    '    dwFlags: Default::default(),',
    '    usUsagePage: 1, usUsage: 6, dwFlags: RIDEV_DEVNOTIFY, hwndTarget: HWND(0x1234 as _),',
  ];
  for (const body of ok) assert.deepEqual(scanSource(rawInput(body)), [], body);
  assert.deepEqual(scanSource(`${rawInput('    usUsage: 6,')}assert!(got.dwFlags == RIDEV_DEVNOTIFY);\n`), []);
  assert.deepEqual(scanSource('SetWindowsHooks(); let x = WH_KEYBOARDS;'), []);
});

test('requires the Always filter, and a commented-out one does not count', () => {
  assert.ok(hasRequiredFilter(LIB));
  assert.ok(hasRequiredFilter(LIB.replace('tauri::DeviceEventFilter', 'DeviceEventFilter')));
  assert.ok(!hasRequiredFilter(LIB.replace(ALWAYS, '')));
  assert.ok(!hasRequiredFilter(LIB.replace(ALWAYS, `// ${ALWAYS}`)));
});

test('flags capture crates', () => {
  const tree = 'keytriage v0.0.0 (C:\\code)\nrdev v0.5.3\nglobal-hotkey v0.7.0\nmultiinput v0.1.0\ntao v0.37.1\n';
  assert.deepEqual(scanCargoTree(tree), ['rdev', 'global-hotkey', 'multiinput']);
});

function withFixture(body) {
  const root = mkdtempSync(join(tmpdir(), 'keytriage-capture-'));
  try {
    mkdirSync(join(root, 'src-tauri/src'), { recursive: true });
    mkdirSync(join(root, 'crates/input/src'), { recursive: true });
    writeFileSync(join(root, 'src-tauri/src/lib.rs'), LIB);
    writeFileSync(join(root, 'crates/input/src/lib.rs'), 'pub fn register() {}\n');
    body(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const run = (root) =>
  spawnSync(process.execPath, [script, root], { encoding: 'utf8', env: { ...process.env, GITHUB_ACTIONS: '' } });

test('the CLI exits 1 on a planted RIDEV_INPUTSINK and 0 without it', () => {
  withFixture((root) => {
    assert.equal(run(root).status, 0);
    writeFileSync(join(root, 'crates/input/src/lib.rs'), 'pub fn register() {\n    let flags = RIDEV_INPUTSINK;\n}\n');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /crates\/input\/src\/lib\.rs:2 {2}background capture: RIDEV_INPUTSINK/);
  });
});

test('the CLI exits 1 on a planted DeviceEventFilter::Never', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'src-tauri/src/lib.rs'), LIB.replace('Always', 'Never'));
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /src-tauri\/src\/lib\.rs:3 {2}background capture: a DeviceEventFilter other than Always/);
    assert.match(result.stderr, /the Builder must set/);
  });
});

test('the CLI scans C and C++ sources too', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'crates/input/src/shim.c'), 'HHOOK h = SetWindowsHookExW(WH_KEYBOARD_LL, proc, 0, 0);\n');
    writeFileSync(join(root, 'crates/input/src/shim.CXX'), 'SHORT s = GetAsyncKeyState(VK_A);\n');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /shim\.c:1 {2}background capture: SetWindowsHookEx/);
    assert.match(result.stderr, /shim\.CXX:1 {2}background capture: GetAsyncKeyState/);
  });
});

test('the CLI fails on the global shortcut plugin in package-lock.json', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'package-lock.json'), '{"packages": {"node_modules/@tauri-apps/plugin-global-shortcut": {}}}');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /plugin @tauri-apps\/plugin-global-shortcut/);
  });
});

test('the CLI runs cargo tree on the workspace and fails on a capture crate', () => {
  withFixture((root) => {
    const crate = (dir, name, extra = '') => {
      mkdirSync(join(root, dir, 'src'), { recursive: true });
      writeFileSync(join(root, dir, 'Cargo.toml'), `[package]\nname = "${name}"\nversion = "0.0.0"\nedition = "2024"\n${extra}`);
      writeFileSync(join(root, dir, 'src/lib.rs'), '\n');
    };
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["app"]\nresolver = "3"\n');
    crate('app', 'app', '[dependencies]\nrdev = { path = "../vendor/rdev" }\n');
    crate('vendor/rdev', 'rdev');
    const lock = spawnSync('cargo', ['generate-lockfile', '--offline'], { cwd: root, encoding: 'utf8' });
    assert.equal(lock.status, 0, lock.stderr);
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /capture crate rdev on x86_64-pc-windows-msvc/);
  });
});
