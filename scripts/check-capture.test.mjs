// Positive controls for the capture guard: each banned thing must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  dependencyCalls,
  hasRequiredFilter,
  scanCargoTree,
  scanDependencies,
  scanSource,
  shippedPackages,
  unlistedDependencyCalls,
} from './check-capture.mjs';

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

test('the CLI skips build folders only where they live', () => {
  const hook = 'fn f() -> i16 { unsafe { GetAsyncKeyState(0x41) } }\n';
  withFixture((root) => {
    for (const dir of ['src-tauri/gen/schemas', 'target/debug', 'dist/x', 'node_modules/x']) {
      mkdirSync(join(root, dir), { recursive: true });
      writeFileSync(join(root, dir, 'out.rs'), hook);
    }
    assert.equal(run(root).status, 0);
    for (const dir of ['crates/input/src/gen', 'crates/input/src/target']) {
      mkdirSync(join(root, dir), { recursive: true });
      writeFileSync(join(root, dir, 'keys.rs'), hook);
    }
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /crates\/input\/src\/gen\/keys\.rs:1 {2}background capture: GetAsyncKeyState/);
    assert.match(result.stderr, /crates\/input\/src\/target\/keys\.rs:1 {2}background capture: GetAsyncKeyState/);
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

test('follows normal and build edges to shipped packages', () => {
  const edge = (pkg, kind) => ({ pkg, dep_kinds: [{ kind }] });
  const meta = {
    workspace_members: ['app'],
    packages: ['app', 'lib', 'build', 'dev', 'deep'].map((id) => ({ id, name: id })),
    resolve: {
      nodes: [
        { id: 'app', deps: [edge('lib', null), edge('build', 'build'), edge('dev', 'dev')] },
        { id: 'lib', deps: [edge('deep', null)] },
        { id: 'build', deps: [] },
        { id: 'dev', deps: [] },
        { id: 'deep', deps: [] },
      ],
    },
  };
  assert.deepEqual(shippedPackages(meta).map((p) => p.id).sort(), ['build', 'deep', 'lib']);
});

function withCrate(files, body) {
  const dir = mkdtempSync(join(tmpdir(), 'keytriage-crate-'));
  try {
    for (const [path, text] of Object.entries(files)) {
      mkdirSync(dirname(join(dir, path)), { recursive: true });
      writeFileSync(join(dir, path), text);
    }
    body(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const byFile = (a, b) => `${a.file} ${a.api}`.localeCompare(`${b.file} ${b.api}`);

test('counts the lines of each dependency file that name a capture API', () => {
  const files = {
    'src/lib.rs':
      '// GetAsyncKeyState in a comment does not count.\n' +
      'fn a() -> i16 { unsafe { GetAsyncKeyState(0x41) } }\n' +
      'fn b() -> i16 { unsafe { GetAsyncKeyState(0x42) } }\n',
    'src/gen/hook.c': 'HHOOK h = SetWindowsHookExW(WH_KEYBOARD_LL, proc, 0, 0);\n',
    'src/target/windows.rs': 'fn c() { RegisterHotKey(h, 1, 0, 0x41); }\n',
    'target/debug/build/out.rs': 'fn d() { RegisterHotKey(h, 2, 0, 0x42); }\n',
  };
  withCrate(files, (dir) => {
    assert.deepEqual(dependencyCalls(dir).sort(byFile), [
      { file: 'src/gen/hook.c', api: 'SetWindowsHookEx', lines: 1 },
      { file: 'src/gen/hook.c', api: 'WH_KEYBOARD_LL', lines: 1 },
      { file: 'src/lib.rs', api: 'GetAsyncKeyState', lines: 2 },
      { file: 'src/target/windows.rs', api: 'RegisterHotKey', lines: 1 },
    ]);
  });
});

test('counts calls, not imports, and still reports an import alone', () => {
  const count = (text) => {
    let found;
    withCrate({ 'src/k.rs': text }, (dir) => (found = dependencyCalls(dir)));
    return found.map((c) => c.lines);
  };
  assert.deepEqual(count('use w::{\n    GetAsyncKeyState,\n    GetKeyState,\n};\nfn f() { GetAsyncKeyState(1); }\n'), [1]);
  assert.deepEqual(count('use w::KeyboardAndMouse::*;\nfn f() { GetAsyncKeyState(1); }\nfn g() { GetAsyncKeyState(2); }\n'), [2]);
  assert.deepEqual(count('pub(crate) use w::GetAsyncKeyState as held;\nfn f() { held(1); }\n'), [0]);
  assert.deepEqual(count('use a::B; let s = GetAsyncKeyState(vk);\n'), [1]);
  assert.deepEqual(count('/*\nuse caution here\n*/\nlet s = GetAsyncKeyState(vk);\n'), [1]);
  // A C file has no use declarations to skip.
  let found;
  withCrate({ 'src/k.c': 'use GetAsyncKeyState;\n' }, (dir) => (found = dependencyCalls(dir)));
  assert.deepEqual(found.map((c) => c.lines), [1]);
});

test('counts the callers of an allowed wrapper', () => {
  const allowed = [{ crate: 'tao', file: 'src/k.rs', api: 'held_keys', lines: 2, wrapper: true }];
  const tao = (text) => {
    let calls;
    withCrate({ 'src/k.rs': text }, (dir) => {
      calls = scanDependencies([{ name: 'tao', version: '1.0.0', source: null, manifest_path: join(dir, 'Cargo.toml') }], allowed);
    });
    return unlistedDependencyCalls(calls, new Set(['tao']), allowed);
  };
  const wrapper = 'fn held_keys() -> u8 { 0 }\n';
  assert.deepEqual(tao(`${wrapper}fn on_focus() { held_keys(); }\n`), []);
  assert.match(tao(`${wrapper}fn on_focus() { held_keys(); }\nfn on_key() { held_keys(); }\n`)[0], /held_keys on 3 line\(s\) of src\/k\.rs, but DEPENDENCY_ALLOWED allows 2/);
});

test('skips only the crates.io releases of the binding crates', () => {
  withCrate({ 'src/lib.rs': 'pub fn f() { RegisterHotKey(h, 1, 0, 0x41); }\n' }, (dir) => {
    const manifest_path = join(dir, 'Cargo.toml');
    const crate = (name, source) => ({ name, version: '1.0.0', source, manifest_path });
    const crateIo = 'registry+https://github.com/rust-lang/crates.io-index';
    const calls = scanDependencies([
      crate('windows', crateIo),
      crate('windows-sys', crateIo),
      crate('windows', null),
      crate('tao', crateIo),
    ]);
    assert.deepEqual(calls.map((c) => c.crate), ['windows', 'tao']);
  });
});

test('fails on unlisted, miscounted and stale dependency calls', () => {
  const allowed = [{ crate: 'tao', file: 'src/k.rs', api: 'GetAsyncKeyState', lines: 2 }];
  const call = (crate, lines, api = 'GetAsyncKeyState') => ({ crate, version: '1.0.0', file: 'src/k.rs', api, lines });
  const tao = new Set(['tao']);

  assert.deepEqual(unlistedDependencyCalls([call('tao', 2)], tao, allowed), []);
  assert.match(unlistedDependencyCalls([call('tao', 3)], tao, allowed)[0], /on 3 line\(s\) of src\/k\.rs, but DEPENDENCY_ALLOWED allows 2/);
  assert.match(unlistedDependencyCalls([call('tao', 1)], tao, allowed)[0], /allows 2/);

  const other = unlistedDependencyCalls([call('tao', 2), call('hooky', 1, 'SetWindowsHookEx')], new Set(['tao', 'hooky']), allowed);
  assert.deepEqual(other.length, 1);
  assert.match(other[0], /hooky 1\.0\.0 names SetWindowsHookEx .*list it in DEPENDENCY_ALLOWED/);

  assert.match(unlistedDependencyCalls([], tao, allowed)[0], /tao no longer names GetAsyncKeyState .*remove it/);
  assert.deepEqual(unlistedDependencyCalls([], new Set(), allowed), []);
});

test('the CLI fails on a capture API in a dependency and passes without it', () => {
  withCrate({ 'Cargo.toml': '[package]\nname = "hooky"\nversion = "0.1.0"\nedition = "2024"\n', 'src/lib.rs': '\n' }, (vendor) => {
    withFixture((root) => {
      const dep = JSON.stringify(vendor.replaceAll('\\', '/'));
      writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["app"]\nresolver = "3"\n');
      mkdirSync(join(root, 'app/src'), { recursive: true });
      writeFileSync(join(root, 'app/Cargo.toml'), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n[dependencies]\nhooky = { path = ${dep} }\n`);
      writeFileSync(join(root, 'app/src/lib.rs'), '\n');
      const lock = spawnSync('cargo', ['generate-lockfile', '--offline'], { cwd: root, encoding: 'utf8' });
      assert.equal(lock.status, 0, lock.stderr);

      const clean = run(root);
      assert.equal(clean.status, 0, clean.stderr);

      writeFileSync(join(vendor, 'src/lib.rs'), 'pub fn held() -> i16 {\n    unsafe { GetAsyncKeyState(0x41) }\n}\n');
      const result = run(root);
      assert.equal(result.status, 1);
      assert.match(result.stderr, /Cargo\.toml:1 {2}dependency capture on x86_64-pc-windows-msvc: hooky 0\.1\.0 names GetAsyncKeyState on 1 line\(s\) of src\/lib\.rs/);
    });
  });
});
