// Network guard: fails when networking code, a network dependency, a remote stylesheet, font,
// script or image, a permissive CSP or a missing navigation guard appears. It enforces the "no
// network" line of the privacy contract in README.md.
//
// Limits: it matches names, not behavior. A dependency that opens sockets under a name not listed
// here, a child process that reaches the network, a URL built at run time, code generated at build
// time and deliberate obfuscation get past it. Review covers those.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';

const WEB_APIS = [
  { pattern: /\bfetch\s*\(/, name: 'fetch(' },
  { pattern: /\bXMLHttpRequest\b/, name: 'XMLHttpRequest' },
  { pattern: /\bWebSocket\b/, name: 'WebSocket' },
  { pattern: /\bEventSource\b/, name: 'EventSource' },
  { pattern: /\bsendBeacon\b/, name: 'sendBeacon' },
  // The CSP does not cover WebRTC. No leading \b, so webkitRTCPeerConnection is caught too.
  { pattern: /RTCPeerConnection\b/, name: 'RTCPeerConnection' },
  { pattern: /@angular\/common\/http/, name: '@angular/common/http' },
];

// Minified npm code can take fetch as a value instead of calling it by name.
export const BUNDLE_APIS = WEB_APIS.map((api) =>
  api.name === 'fetch(' ? { pattern: /\bfetch\b/, name: 'fetch' } : api,
);

// The CSP stops a remote stylesheet or font only inside the app. `npm start` serves the page to a
// browser with no CSP. The minifier drops the space in `@import "…"`, and image-set() takes bare
// strings as well as url().
const REMOTE = String.raw`(?:https?:|wss?:|\/\/)`;
const REMOTE_CSS = new RegExp(
  String.raw`(?:@import\s*(?:url\(\s*)?|\burl\(\s*)['"]?\s*${REMOTE}|image-set\([^)]*?['"]\s*${REMOTE}`,
  'gi',
);

// The same holds for the page's markup, where a pasted Google Fonts <link> is the usual way in. A
// plain <a href> passes, because links open in the browser. Tags span lines, so the whole text is
// matched at once. `[src]="'…'"` is Angular's bound form.
const REMOTE_MARKUP = [
  new RegExp(String.raw`<(?:link|use|image)\b[^>]*?\s\[?(?:xlink:)?href\]?\s*=\s*['"]{0,2}\s*${REMOTE}`, 'gi'),
  new RegExp(String.raw`\b(?:src|poster)\]?\s*=\s*['"]{0,2}\s*${REMOTE}`, 'gi'),
  // Every candidate in a srcset is a URL.
  new RegExp(String.raw`\bsrcset\]?\s*=\s*['"]{1,2}[^'"]*?${REMOTE}`, 'gi'),
];

export const RUST_APIS = [
  { pattern: /\bstd::net\b/, name: 'std::net' },
  { pattern: /\b(TcpStream|TcpListener|UdpSocket|ToSocketAddrs)\b/, name: 'socket type' },
];

const NETWORK_CRATES = new Set([
  'async-tungstenite', 'attohttpc', 'awc', 'curl', 'h2', 'hyper', 'isahc', 'minreq', 'mio',
  'native-tls', 'openssl', 'reqwest', 'rustls', 'socket2', 'surf', 'tiny_http',
  'tokio-tungstenite', 'tungstenite', 'ureq',
  'tauri-plugin-http', 'tauri-plugin-localhost', 'tauri-plugin-updater', 'tauri-plugin-upload',
  'tauri-plugin-websocket',
  // It lets the page run programs, and those can reach the network outside the CSP.
  'tauri-plugin-shell',
]);

// The windows crates carry WinHTTP, WinINet, Winsock and WinRT networking behind features, so the
// crate name alone says nothing.
const WINDOWS_CRATES = new Set(['windows', 'windows-sys']);
const WINDOWS_NETWORK_FEATURE = /^(Web|Networking|Win32_Networking|Win32_Web|Win32_System_Com_Urlmon)(_|$)/;
const MANIFEST_NETWORK_FEATURE = /"(Web|Networking|Win32_Networking|Win32_Web|Win32_System_Com_Urlmon)(_\w*)?"/;

// Cargo.lock lists crates for every platform (tauri pulls reqwest on mobile only), so ask cargo
// what the shipped target compiles. Add Linux here when it becomes a target.
export const TARGETS = ['x86_64-pc-windows-msvc'];

const NPM_PLUGINS = /node_modules\/@tauri-apps\/plugin-(http|shell|updater|upload|websocket)"/g;

// IPC is Tauri's in-process channel. On Windows it is served as http://ipc.localhost and never
// leaves the machine.
const ALLOWED_CONNECT = new Set(['ipc:', 'http://ipc.localhost']);

const SKIP_DIRS = new Set(['.git', '.angular', 'dist', 'gen', 'node_modules', 'target']);

export function scanSource(text, apis = WEB_APIS) {
  const hits = [];
  text.split('\n').forEach((line, i) => {
    // Comments that explain the privacy contract name the APIs it bans.
    if (apis !== BUNDLE_APIS && /^\s*\/\//.test(line)) return;
    for (const api of apis) {
      const match = api.pattern.exec(line);
      if (!match) continue;
      const hit = { line: i + 1, name: api.name };
      // A minified bundle line is huge, so show where in it the name sits.
      if (apis === BUNDLE_APIS) {
        hit.context = line.slice(Math.max(0, match.index - 40), match.index + 40).trim();
      }
      hits.push(hit);
    }
  });
  return hits;
}

export function scanCss(text) {
  const hits = [];
  text.split('\n').forEach((line, i) => {
    for (const match of line.matchAll(REMOTE_CSS)) {
      const context = line.slice(Math.max(0, match.index - 40), match.index + 60).trim();
      hits.push({ line: i + 1, name: 'remote CSS import or url', context });
    }
  });
  return hits;
}

export function scanMarkup(text) {
  const hits = [];
  for (const pattern of REMOTE_MARKUP) {
    for (const match of text.matchAll(pattern)) {
      const end = match.index + match[0].length;
      const line = text.slice(0, end).split('\n').length;
      const context = text.slice(Math.max(0, end - 60), end + 40).replace(/\s+/g, ' ').trim();
      hits.push({ line, name: 'remote file in markup', context });
    }
  }
  return hits.sort((a, b) => a.line - b.line);
}

export function scanCargoTree(text) {
  const problems = new Set();
  for (const line of text.split('\n')) {
    const [pkg, rawFeatures = ''] = line.split('|');
    const name = pkg.split(' ')[0];
    const features = rawFeatures.replace(' (*)', '').trim().split(',').filter(Boolean);
    if (NETWORK_CRATES.has(name)) problems.add(`network crate ${name}`);
    if (name === 'tokio' && features.includes('net')) problems.add('tokio with its net feature');
    if (WINDOWS_CRATES.has(name)) {
      for (const f of features) {
        if (WINDOWS_NETWORK_FEATURE.test(f)) problems.add(`${name} networking feature ${f}`);
      }
    }
  }
  return [...problems];
}

export function scanPackageLock(text) {
  return [...new Set([...text.matchAll(NPM_PLUGINS)].map((m) => `@tauri-apps/plugin-${m[1]}`))];
}

export function checkCsp(csp) {
  if (typeof csp !== 'string' || !csp.trim()) return ['app.security.csp is not set'];
  const directives = new Map(
    csp
      .split(';')
      .map((d) => d.trim().split(/\s+/))
      .filter((d) => d[0])
      .map(([k, ...v]) => [k, v]),
  );
  const problems = [];
  if (!directives.get('default-src')?.every((s) => s === "'self'" || s === "'none'")) {
    problems.push("default-src must be 'self' or 'none'");
  }
  const connect = directives.get('connect-src');
  if (!connect) problems.push('connect-src is missing');
  else for (const s of connect) if (!ALLOWED_CONNECT.has(s)) problems.push(`connect-src allows ${s}`);
  for (const [name, sources] of directives) {
    if (name === 'connect-src') continue;
    for (const s of sources) {
      if (s === '*' || /^(https?|wss?):/.test(s)) problems.push(`${name} allows ${s}`);
    }
  }
  return problems;
}

export function hasNavigationGuard(libRs) {
  const code = libRs.replace(/\/\/.*$/gm, '');
  return /\.plugin\(\s*navigation_guard\(\s*\)\s*\)/.test(code);
}

function filesUnder(dir, exts) {
  if (!existsSync(dir)) return [];
  const found = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) found.push(...filesUnder(path, exts));
    } else if (exts.includes(extname(entry.name)) || exts.includes(entry.name)) {
      found.push(path);
    }
  }
  return found;
}

export function cargoTree(root, target) {
  const args = [
    'tree', '--workspace', '--locked', '--target', target, '--edges', 'normal,build',
    '--prefix', 'none', '--format', '{p}|{f}',
  ];
  return spawnSync('cargo', args, { cwd: root, encoding: 'utf8' });
}

export function check(root, { requireBundle = false } = {}) {
  const errors = [];
  const report = (file, line, message) =>
    errors.push({ file: relative(root, file).replaceAll('\\', '/') || '.', line, message });

  // Components and templates can hold their styles inline, and component styles compile into the
  // JavaScript bundle, so the CSS scan reads every file the page is built from.
  for (const file of filesUnder(join(root, 'src'), ['.ts', '.js', '.mjs', '.html', '.css'])) {
    const text = readFileSync(file, 'utf8');
    if (extname(file) !== '.css') {
      for (const hit of scanSource(text)) report(file, hit.line, `networking API ${hit.name}`);
      for (const hit of scanMarkup(text)) report(file, hit.line, hit.name);
    }
    for (const hit of scanCss(text)) report(file, hit.line, hit.name);
  }

  const conf = join(root, 'src-tauri/tauri.conf.json');
  const config = existsSync(conf) ? JSON.parse(readFileSync(conf, 'utf8')) : null;
  if (!config) report(conf, 1, 'tauri.conf.json is missing');
  else for (const problem of checkCsp(config.app?.security?.csp)) report(conf, 1, `CSP: ${problem}`);

  const bundle = config?.build?.frontendDist && resolve(root, 'src-tauri', config.build.frontendDist);
  if (bundle && existsSync(bundle)) {
    // A development build carries Angular's and Tauri's debug code, which names these APIs.
    if (filesUnder(bundle, ['.map']).length) {
      report(bundle, 1, 'the built bundle is a development build; run npm run build first');
    } else {
      for (const file of filesUnder(bundle, ['.js', '.mjs', '.html', '.css'])) {
        const text = readFileSync(file, 'utf8');
        if (extname(file) !== '.css') {
          for (const hit of scanSource(text, BUNDLE_APIS)) {
            report(file, hit.line, `networking API ${hit.name} in the built bundle, near: ${hit.context}`);
          }
          for (const hit of scanMarkup(text)) {
            report(file, hit.line, `${hit.name} in the built bundle, near: ${hit.context}`);
          }
        }
        for (const hit of scanCss(text)) {
          report(file, hit.line, `${hit.name} in the built bundle, near: ${hit.context}`);
        }
      }
    }
  } else if (requireBundle) {
    report(bundle ?? conf, 1, 'the built bundle is missing; run npm run build first');
  }

  const lib = join(root, 'src-tauri/src/lib.rs');
  if (existsSync(lib) && !hasNavigationGuard(readFileSync(lib, 'utf8'))) {
    report(lib, 1, 'lib.rs must register .plugin(navigation_guard())');
  }

  for (const file of filesUnder(root, ['.rs'])) {
    for (const hit of scanSource(readFileSync(file, 'utf8'), RUST_APIS)) {
      report(file, hit.line, `networking API ${hit.name}`);
    }
  }
  for (const file of filesUnder(root, ['Cargo.toml'])) {
    readFileSync(file, 'utf8').split('\n').forEach((line, i) => {
      if (MANIFEST_NETWORK_FEATURE.test(line)) report(file, i + 1, 'Windows networking feature');
    });
  }

  const manifest = join(root, 'Cargo.toml');
  if (existsSync(manifest)) {
    for (const target of TARGETS) {
      const run = cargoTree(root, target);
      if (run.status !== 0) {
        report(manifest, 1, `cargo tree failed for ${target}: ${(run.stderr || String(run.error)).trim()}`);
      } else {
        for (const problem of scanCargoTree(run.stdout)) report(manifest, 1, `${problem} on ${target}`);
      }
    }
  }

  const lock = join(root, 'package-lock.json');
  if (existsSync(lock)) {
    for (const plugin of scanPackageLock(readFileSync(lock, 'utf8'))) report(lock, 1, `plugin ${plugin}`);
  }

  return errors;
}

// GitHub reads one workflow command per line.
const escape = (s) => String(s).replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A');

if (import.meta.main) {
  const args = process.argv.slice(2);
  const root = resolve(args.find((a) => !a.startsWith('--')) ?? '.');
  const errors = check(root, { requireBundle: args.includes('--bundle') });
  for (const e of errors) {
    console.error(
      process.env.GITHUB_ACTIONS
        ? `::error file=${e.file},line=${e.line}::${escape(e.message)}`
        : `${e.file}:${e.line}  ${e.message}`,
    );
  }
  if (errors.length) {
    console.error(`\nNetwork check failed: ${errors.length} problem(s). keytriage has no networking code.`);
    process.exit(1);
  }
  console.log('Network check passed.');
}
