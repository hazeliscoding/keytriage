// Positive controls for the network guard: each banned thing must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  BUNDLE_APIS,
  RUST_APIS,
  cargoTree,
  checkCsp,
  hasNavigationGuard,
  scanCargoTree,
  scanCss,
  scanPackageLock,
  scanSource,
} from './check-network.mjs';

const CSP = "default-src 'self'; connect-src ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'";
const GUARDED_LIB = 'pub fn run() {\n    tauri::Builder::default()\n        .plugin(navigation_guard())\n        .run(ctx);\n}\n';
const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const script = join(repo, 'scripts/check-network.mjs');

test('flags each browser networking API', () => {
  const samples = [
    "fetch('https://example.com')",
    'await window.fetch (url)',
    'const x = new XMLHttpRequest();',
    'const ws = new WebSocket(url);',
    'const es = new EventSource(url);',
    'navigator.sendBeacon(url, body);',
    'const pc = new RTCPeerConnection();',
    'const pc = new webkitRTCPeerConnection();',
    "import { HttpClient } from '@angular/common/http';",
    "load(); // then fetch('x')",
  ];
  for (const s of samples) assert.equal(scanSource(s).length, 1, s);
});

test('ignores lookalikes and whole-line comments', () => {
  assert.deepEqual(scanSource('prefetch(); const fetched = true; // WebSockets2'), []);
  assert.deepEqual(scanSource('  // No fetch( or WebSocket: the app has no network.'), []);
});

test('the bundle scan catches fetch as a value and shows where', () => {
  const [hit] = scanSource('var a=1;var f=fetch;f(u);var b=2;', BUNDLE_APIS);
  assert.equal(hit.name, 'fetch');
  assert.match(hit.context, /var f=fetch;f\(u\)/);
  assert.equal(scanSource('// fetch', BUNDLE_APIS).length, 1);
});

test('reports the line number', () => {
  assert.deepEqual(scanSource('const a = 1;\n\nfetch(u);'), [{ line: 3, name: 'fetch(' }]);
});

test('flags each remote CSS import and url once', () => {
  const samples = [
    "@import url('https://fonts.googleapis.com/css2?family=Public+Sans');",
    "@import 'http://x/y.css';",
    '@import"https://fonts.googleapis.com/css2?family=Public+Sans";',
    '@IMPORT URL( "wss://x/y.css" );',
    'src: url(//cdn.example.com/x.woff2) format("woff2");',
    'background: url("https://x/y.woff2");',
    "styles: ['.a { background: url( ws://x/y.png ) }'],",
  ];
  for (const s of samples) assert.equal(scanCss(s).length, 1, s);
});

test('ignores local files, data URIs and lookalikes in CSS', () => {
  const samples = [
    'src: url(./files/public-sans-latin-400-normal.woff2) format("woff2");',
    'src: url(media/x.woff2);',
    "@import './tokens.css';",
    `.radio { background: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>"); }`,
    "const page = curl('https://example.com');",
  ];
  for (const s of samples) assert.deepEqual(scanCss(s), [], s);
});

test('the CSS scan reports the line and shows where in a minified line', () => {
  assert.deepEqual(scanCss('a{}\n\n@import "https://x/y.css";').map((h) => h.line), [3]);
  const [hit, ...rest] = scanCss('@font-face{font-family:A;src:url(//cdn.example.com/x.woff2) format("woff2"),url(./a.woff2)}');
  assert.deepEqual(rest, []);
  assert.match(hit.context, /src:url\(\/\/cdn\.example\.com\/x\.woff2\)/);
});

test('flags Rust sockets, but not in comments', () => {
  assert.equal(scanSource('use std::net::TcpStream;', RUST_APIS).length, 2);
  assert.equal(scanSource('let s = UdpSocket::bind(a);', RUST_APIS).length, 1);
  assert.equal(scanSource('let a = "h:1".to_socket_addrs(); use std::net::ToSocketAddrs;', RUST_APIS).length, 2);
  assert.deepEqual(scanSource('    /// Never opens a TcpStream or uses std::net.', RUST_APIS), []);
});

test('flags network crates, tokio sockets, windows networking features and plugins', () => {
  const tree = 'keytriage v0.0.0 (C:\\code)|\ntauri v2.12.0|default\nreqwest v0.13.5|\nhyper v1.11.1|\nreqwest v0.13.5| (*)\n';
  assert.deepEqual(scanCargoTree(tree), ['network crate reqwest', 'network crate hyper']);
  assert.deepEqual(scanCargoTree('tauri-plugin-updater v2.0.0|\n'), ['network crate tauri-plugin-updater']);
  assert.deepEqual(scanCargoTree('tauri-plugin-shell v2.4.0|\n'), ['network crate tauri-plugin-shell']);
  assert.deepEqual(scanCargoTree('tiny_http v0.12.0|default\n'), ['network crate tiny_http']);
  assert.deepEqual(scanCargoTree('tokio v1.53.1|bytes,net,rt\n'), ['tokio with its net feature']);
  assert.deepEqual(scanCargoTree('windows v0.62.2|Web,Web_Http,default\n'), [
    'windows networking feature Web',
    'windows networking feature Web_Http',
  ]);
  assert.deepEqual(scanCargoTree('windows-sys v0.61.2|Win32,Win32_Networking_WinSock,default (*)\n'), [
    'windows-sys networking feature Win32_Networking_WinSock',
  ]);
  assert.deepEqual(
    scanCargoTree('windows v0.62.2|Win32_UI_Input,Win32_Foundation,Win32_System_Com\ntokio v1.53.1|bytes,rt,sync\nhttp v1.5.0|\n'),
    [],
  );
  assert.deepEqual(scanPackageLock('"node_modules/@tauri-apps/plugin-http": {'), ['@tauri-apps/plugin-http']);
  assert.deepEqual(scanPackageLock('"node_modules/@tauri-apps/plugin-shell": {'), ['@tauri-apps/plugin-shell']);
});

test('the guard runs cargo tree with the features it scans: the Android tree has reqwest and tokio sockets', () => {
  const run = cargoTree(repo, 'aarch64-linux-android');
  assert.equal(run.status, 0, run.stderr);
  const problems = scanCargoTree(run.stdout);
  assert.ok(problems.includes('network crate reqwest'), problems.join(', '));
  assert.ok(problems.includes('tokio with its net feature'), problems.join(', '));
});

test('accepts the app CSP and rejects remote sources', () => {
  assert.deepEqual(checkCsp(CSP), []);
  assert.ok(checkCsp(null).length);
  assert.ok(checkCsp("default-src 'self'").some((p) => p.includes('connect-src is missing')));
  assert.ok(checkCsp(`${CSP} https://api.example.com`).some((p) => p.includes('https://api.example.com')));
  assert.ok(checkCsp(`${CSP}; img-src *`).some((p) => p.includes('img-src allows *')));
  assert.ok(checkCsp("default-src *; connect-src ipc:").length);
});

test('requires the navigation guard, and a commented-out one does not count', () => {
  assert.ok(hasNavigationGuard(GUARDED_LIB));
  assert.ok(!hasNavigationGuard(GUARDED_LIB.replace('.plugin(navigation_guard())', '')));
  assert.ok(!hasNavigationGuard(GUARDED_LIB.replace('.plugin', '// .plugin')));
});

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'keytriage-network-'));
  mkdirSync(join(root, 'src/app'), { recursive: true });
  mkdirSync(join(root, 'src-tauri/src'), { recursive: true });
  mkdirSync(join(root, 'dist/app/browser'), { recursive: true });
  const conf = { build: { frontendDist: '../dist/app/browser' }, app: { security: { csp: CSP } } };
  writeFileSync(join(root, 'src-tauri/tauri.conf.json'), JSON.stringify(conf));
  writeFileSync(join(root, 'src-tauri/src/lib.rs'), GUARDED_LIB);
  writeFileSync(join(root, 'src/app/ok.ts'), 'export const ok = 1;\n');
  writeFileSync(join(root, 'dist/app/browser/main.js'), 'console.log(1);\n');
  return root;
}

function withFixture(body) {
  const root = fixture();
  try {
    body(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const run = (root, ...args) =>
  spawnSync(process.execPath, [script, root, ...args], { encoding: 'utf8', env: { ...process.env, GITHUB_ACTIONS: '' } });

test('the CLI exits 1 on a planted fetch( and 0 without it', () => {
  withFixture((root) => {
    assert.equal(run(root, '--bundle').status, 0);
    writeFileSync(join(root, 'src/app/leak.ts'), "export const leak = () =>\n  fetch('https://example.com');\n");
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /src\/app\/leak\.ts:2 {2}networking API fetch\(/);
  });
});

test('the CLI fails on networking an npm package brought into the bundle', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'dist/app/browser/main.js'), 'var f=fetch;f(u);\n');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /networking API fetch in the built bundle, near: var f=fetch/);
  });
});

test('the CLI fails on a remote import in the built CSS and passes local fonts', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'dist/app/browser/styles.css'), '@font-face{font-family:A;src:url(media/a.woff2)}\n');
    assert.equal(run(root, '--bundle').status, 0);
    writeFileSync(join(root, 'dist/app/browser/styles.css'), '@import"https://fonts.googleapis.com/css2?family=Public+Sans";\n');
    const result = run(root, '--bundle');
    assert.equal(result.status, 1);
    assert.match(
      result.stderr,
      /dist\/app\/browser\/styles\.css:1 {2}remote CSS import or url in the built bundle, near: @import"https:\/\/fonts/,
    );
  });
});

test('the CLI fails on remote CSS in source styles and in styles compiled into the bundle', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'src/styles.css'), "@import url('https://fonts.googleapis.com/css2?family=Public+Sans');\n");
    writeFileSync(join(root, 'src/app/card.ts'), "export const styles = [\n  '.a { background: url(//cdn.example.com/a.png) }',\n];\n");
    writeFileSync(join(root, 'dist/app/browser/main.js'), 'var s=[".a[_ngcontent-%COMP%]{background:url(https://x/y.png)}"];\n');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /src\/styles\.css:1 {2}remote CSS import or url\n/);
    assert.match(result.stderr, /src\/app\/card\.ts:2 {2}remote CSS import or url\n/);
    assert.match(result.stderr, /main\.js:1 {2}remote CSS import or url in the built bundle, near: .*background:url\(https:/);
  });
});

test('the CLI refuses a development bundle, and a missing one only when --bundle asks for it', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'dist/app/browser/main.js.map'), '{}');
    assert.match(run(root).stderr, /the built bundle is a development build/);
    rmSync(join(root, 'dist'), { recursive: true });
    assert.equal(run(root).status, 0);
    assert.match(run(root, '--bundle').stderr, /the built bundle is missing/);
  });
});

test('the CLI fails on a permissive CSP and on a network plugin in package-lock.json', () => {
  withFixture((root) => {
    const conf = { build: { frontendDist: '../dist/app/browser' }, app: { security: { csp: `${CSP}; connect-src *` } } };
    writeFileSync(join(root, 'src-tauri/tauri.conf.json'), JSON.stringify(conf));
    writeFileSync(join(root, 'package-lock.json'), '{"packages": {"node_modules/@tauri-apps/plugin-http": {}}}');
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /CSP: connect-src allows \*/);
    assert.match(result.stderr, /plugin @tauri-apps\/plugin-http/);
  });
});

test('the CLI fails on a Rust socket, a Windows networking feature and a removed navigation guard', () => {
  withFixture((root) => {
    mkdirSync(join(root, 'crates/input/src'), { recursive: true });
    writeFileSync(join(root, 'crates/input/src/lib.rs'), '// Sockets are banned.\nuse std::net::TcpStream;\n');
    writeFileSync(join(root, 'crates/input/Cargo.toml'), '[dependencies]\nwindows = { version = "0.62", features = ["Win32_UI_Input", "Networking_Sockets"] }\n');
    writeFileSync(join(root, 'src-tauri/src/lib.rs'), GUARDED_LIB.replace('.plugin(navigation_guard())', ''));
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /crates\/input\/src\/lib\.rs:2 {2}networking API std::net/);
    assert.doesNotMatch(result.stderr, /crates\/input\/src\/lib\.rs:1 /);
    assert.match(result.stderr, /crates\/input\/Cargo\.toml:2 {2}Windows networking feature/);
    assert.match(result.stderr, /lib\.rs must register \.plugin\(navigation_guard\(\)\)/);
  });
});

function crate(root, dir, name, extra = '') {
  mkdirSync(join(root, dir, 'src'), { recursive: true });
  writeFileSync(join(root, dir, 'Cargo.toml'), `[package]\nname = "${name}"\nversion = "0.0.0"\nedition = "2024"\n${extra}`);
  writeFileSync(join(root, dir, 'src/lib.rs'), '\n');
}

test('the CLI runs cargo tree on the workspace and fails on network crates and features', () => {
  withFixture((root) => {
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["app"]\nresolver = "3"\n');
    crate(root, 'app', 'app', [
      '[dependencies]',
      'reqwest = { path = "../vendor/reqwest" }',
      'tokio = { path = "../vendor/tokio", features = ["net"] }',
      'windows = { path = "../vendor/windows", features = ["Web_Http"] }',
      '',
    ].join('\n'));
    crate(root, 'vendor/reqwest', 'reqwest');
    crate(root, 'vendor/tokio', 'tokio', '[features]\nnet = []\n');
    crate(root, 'vendor/windows', 'windows', '[features]\nWeb_Http = []\n');
    const lock = spawnSync('cargo', ['generate-lockfile', '--offline'], { cwd: root, encoding: 'utf8' });
    assert.equal(lock.status, 0, lock.stderr);
    const result = run(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /network crate reqwest on x86_64-pc-windows-msvc/);
    assert.match(result.stderr, /tokio with its net feature on x86_64-pc-windows-msvc/);
    assert.match(result.stderr, /windows networking feature Web_Http on x86_64-pc-windows-msvc/);
  });
});
