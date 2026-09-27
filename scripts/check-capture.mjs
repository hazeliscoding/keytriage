// Capture guard: fails when code could read keys outside a focused test window.
// It enforces the "foreground capture only" line of the privacy contract in README.md.
//
// Limits: it matches names, not behavior. A dependency that captures under a name not listed here,
// code generated at build time and deliberate obfuscation get past it. Review covers those.
// GetKeyState and GetKeyboardState are allowed: they read the calling thread's own input state.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, extname, join, relative, resolve } from 'node:path';

const CAPTURE_APIS = [
  { pattern: /\bRIDEV_INPUTSINK\b/, name: 'RIDEV_INPUTSINK' },
  { pattern: /\bRIDEV_EXINPUTSINK\b/, name: 'RIDEV_EXINPUTSINK' },
  { pattern: /\bSetWindowsHookEx[AW]?\b/, name: 'SetWindowsHookEx' },
  { pattern: /\bWH_KEYBOARD(_LL)?\b/, name: 'WH_KEYBOARD_LL' },
  // These read keys while another app has focus.
  { pattern: /\bGetAsyncKeyState\b/, name: 'GetAsyncKeyState' },
  { pattern: /\bRegisterHotKey\b/, name: 'RegisterHotKey' },
  { pattern: /\bDirectInput8Create\b/, name: 'DirectInput8Create' },
  { pattern: /\bDISCL_BACKGROUND\b/, name: 'DISCL_BACKGROUND' },
];

const BANNED = [
  ...CAPTURE_APIS,
  // tao registers Raw Input for every keyboard unless the filter is Always, and Never adds an input
  // sink without naming it.
  { pattern: /\bDeviceEventFilter\b.*\b(Never|Unfocused)\b/, name: 'a DeviceEventFilter other than Always' },
  { pattern: /\bset_device_event_filter\b/, name: 'set_device_event_filter' },
  {
    pattern: /\bdevice_event_filter\s*\(\s*(?!(tauri::)?DeviceEventFilter::Always\s*\))/,
    name: 'a DeviceEventFilter other than Always',
  },
];

// A sink flag can hide behind a const, a shift or a sum, so Raw Input flags may only be built
// from the named foreground flags, or 0.
const RAW_INPUT_FILE = /\b(RAWINPUTDEVICE|RegisterRawInputDevices)\b/;
const FLAGS_ASSIGNMENT = /\bdwFlags\b(?:\.0)?\s*(?::|\|=|=(?!=))/g;
const SAFE_FLAG_TOKENS = new Set([
  'RIDEV_APPKEYS', 'RIDEV_CAPTUREMOUSE', 'RIDEV_DEVNOTIFY', 'RIDEV_EXCLUDE', 'RIDEV_NOHOTKEYS',
  'RIDEV_NOLEGACY', 'RIDEV_PAGEONLY', 'RIDEV_REMOVE', 'RAWINPUTDEVICE_FLAGS', 'as', 'u32',
  'default',
]);

const REQUIRED_FILTER = /\.device_event_filter\(\s*(?:tauri::)?DeviceEventFilter::Always\s*\)/;

const CAPTURE_CRATES = new Set([
  'device_query', 'global-hotkey', 'inputbot', 'interception', 'livesplit-hotkey', 'mki',
  'multiinput', 'rdev', 'tauri-plugin-global-shortcut', 'willhook', 'win-hotkeys', 'winput',
]);

// Dependency sources are scanned for the capture APIs too, so an upgrade can't add a call unseen.
// Each known call names its file and how many lines outside `use` declarations name the API, so a
// new call in the same file fails as well. A `wrapper` entry counts a crate's own function that
// makes the call, so a new caller fails too. Keep a why comment on each entry.
export const DEPENDENCY_ALLOWED = [
  // It checks which keys are already down when its window gains focus, and stores nothing.
  // Accepted in ROADMAP.md (Raw Input, 2026.09.27). The wrapper's two lines are its definition
  // and its one caller, the WM_SETFOCUS handler.
  { crate: 'tao', file: 'src/platform_impl/windows/keyboard.rs', api: 'GetAsyncKeyState', lines: 1 },
  { crate: 'tao', file: 'src/platform_impl/windows/keyboard.rs', api: 'get_async_kbd_state', lines: 2, wrapper: true },
  // Only DeviceEventFilter::Never reaches it, and this guard requires Always.
  { crate: 'tao', file: 'src/platform_impl/windows/raw_input.rs', api: 'RIDEV_INPUTSINK', lines: 1 },
];

// A `use` names an API without calling it, and import style changes between releases (tao 0.35
// imported KeyboardAndMouse::*, 0.37 lists GetAsyncKeyState), so imports aren't counted. A file
// that names an API only in imports is still reported, with 0 lines, because an alias can hide
// the calls. The pattern can't span a `(`, so it never blanks a call.
const USE_DECL = /^[ \t]*(?:pub(?:\([\w\s:]*\))?[ \t]+)?use[ \t]+[\w\s:{},*#]*;/gm;
const blankUses = (code) => code.replace(USE_DECL, (u) => u.replace(/[^\n]/g, ' '));

// These declare the whole Windows API without calling it, so every banned name appears in them.
// Only their crates.io releases are skipped.
const BINDING_CRATES = new Set(['windows', 'windows-sys']);
const CRATES_IO = 'registry+https://github.com/rust-lang/crates.io-index';

const NPM_PLUGINS = /node_modules\/@tauri-apps\/plugin-global-shortcut"/;

export const TARGETS = ['x86_64-pc-windows-msvc'];

const NATIVE_EXTS = [
  '.rs', '.c', '.cc', '.cpp', '.cxx', '.c++', '.h', '.hh', '.hpp', '.hxx', '.inl', '.ixx',
  '.asm', '.s',
];
// Build output and installed packages, by path: a folder with the same name elsewhere can hold
// source, as cc's src/target does.
const SKIP_PATHS = new Set(['.angular', 'dist', 'node_modules', 'src-tauri/gen', 'target']);
const DEPENDENCY_SKIP_PATHS = new Set(['target']);

function parseNumber(token) {
  const digits = token.replaceAll('_', '').toLowerCase().replace(/[ui](8|16|32|64|size)$/, '');
  if (digits.startsWith('0x')) return parseInt(digits.slice(2), 16);
  if (digits.startsWith('0b')) return parseInt(digits.slice(2), 2);
  if (digits.startsWith('0o')) return parseInt(digits.slice(2), 8);
  return parseInt(digits, 10);
}

function expressionAt(text, start) {
  let depth = 0;
  let end = start;
  for (; end < text.length; end++) {
    const c = text[end];
    if ('([{'.includes(c)) depth++;
    else if (')]}'.includes(c)) {
      if (depth === 0) break;
      depth--;
    } else if ((c === ',' || c === ';') && depth === 0) break;
  }
  return text.slice(start, end);
}

export function unsafeFlagTokens(expression) {
  const bare = expression.replace(/\b[A-Za-z_]\w*::/g, '');
  const tokens = bare.match(/[A-Za-z_]\w*|\d\w*/g) ?? [];
  return tokens.filter((t) => (/^\d/.test(t) ? parseNumber(t) !== 0 : !SAFE_FLAG_TOKENS.has(t)));
}

// Comments that explain the privacy contract name the APIs it bans. Blank them in place so
// offsets and line numbers stay the same.
const blankComments = (text) => text.replace(/^[ \t]*\/\/.*$/gm, (c) => ' '.repeat(c.length));

function apiHits(code, rules) {
  const hits = [];
  code.split('\n').forEach((line, i) => {
    for (const rule of rules) {
      if (rule.pattern.test(line)) hits.push({ line: i + 1, name: rule.name });
    }
  });
  return hits;
}

export function scanSource(text) {
  const code = blankComments(text);
  const hits = apiHits(code, BANNED);
  if (RAW_INPUT_FILE.test(code)) {
    for (const match of code.matchAll(FLAGS_ASSIGNMENT)) {
      const expression = expressionAt(code, match.index + match[0].length);
      const bad = unsafeFlagTokens(expression);
      if (bad.length) {
        const line = code.slice(0, match.index).split('\n').length;
        hits.push({ line, name: `dwFlags built from ${bad.join(', ')}; use named foreground RIDEV_ flags` });
      }
    }
  }
  return hits.sort((a, b) => a.line - b.line);
}

export function hasRequiredFilter(libRs) {
  return REQUIRED_FILTER.test(libRs.replace(/\/\/.*$/gm, ''));
}

export function scanCargoTree(text) {
  return [...new Set(text.split('\n').map((line) => line.split(' ')[0]))].filter((n) =>
    CAPTURE_CRATES.has(n),
  );
}

function filesUnder(dir, skip, base = dir) {
  if (!existsSync(dir)) return [];
  const found = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      const rel = relative(base, path).replaceAll('\\', '/');
      if (entry.name !== '.git' && !skip.has(rel)) found.push(...filesUnder(path, skip, base));
    } else if (NATIVE_EXTS.includes(extname(entry.name).toLowerCase())) {
      found.push(path);
    }
  }
  return found;
}

export function cargoTree(root, target) {
  const args = [
    'tree', '--workspace', '--locked', '--target', target, '--edges', 'normal,build',
    '--prefix', 'none', '--format', '{p}',
  ];
  return spawnSync('cargo', args, { cwd: root, encoding: 'utf8' });
}

export function cargoMetadata(root, target) {
  const args = ['metadata', '--format-version', '1', '--locked', '--filter-platform', target];
  return spawnSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });
}

// The packages the workspace compiles through normal and build edges, without its own members,
// which the stricter source scan covers. Dev-dependencies don't ship.
export function shippedPackages(metadata) {
  const members = new Set(metadata.workspace_members);
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set();
  const stack = [...members];
  while (stack.length) {
    const id = stack.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some((k) => k.kind !== 'dev')) stack.push(dep.pkg);
    }
  }
  return metadata.packages.filter((p) => seen.has(p.id) && !members.has(p.id));
}

export function dependencyCalls(dir, rules = CAPTURE_APIS) {
  const counts = new Map();
  for (const file of filesUnder(dir, DEPENDENCY_SKIP_PATHS)) {
    const rel = relative(dir, file).replaceAll('\\', '/');
    const code = blankComments(readFileSync(file, 'utf8'));
    const key = (hit) => JSON.stringify([rel, hit.name]);
    for (const hit of apiHits(code, rules)) counts.set(key(hit), 0);
    const calls = extname(file).toLowerCase() === '.rs' ? blankUses(code) : code;
    for (const hit of apiHits(calls, rules)) counts.set(key(hit), counts.get(key(hit)) + 1);
  }
  return [...counts].map(([key, lines]) => {
    const [file, api] = JSON.parse(key);
    return { file, api, lines };
  });
}

export function scanDependencies(packages, allowed = DEPENDENCY_ALLOWED) {
  const calls = [];
  for (const pkg of packages) {
    if (BINDING_CRATES.has(pkg.name) && pkg.source === CRATES_IO) continue;
    const wrappers = allowed
      .filter((a) => a.wrapper && a.crate === pkg.name)
      .map((a) => ({ pattern: new RegExp(`\\b${a.api}\\b`), name: a.api }));
    for (const call of dependencyCalls(dirname(pkg.manifest_path), [...CAPTURE_APIS, ...wrappers])) {
      calls.push({ crate: pkg.name, version: pkg.version, ...call });
    }
  }
  return calls;
}

// An entry for a crate that left the tree is not reported, so fixtures without tao stay clean.
export function unlistedDependencyCalls(calls, crates, allowed = DEPENDENCY_ALLOWED) {
  const same = (a, b) => a.crate === b.crate && a.file === b.file && a.api === b.api;
  const problems = [];
  for (const call of calls) {
    const entry = allowed.find((a) => same(a, call));
    const where = `${call.crate} ${call.version} names ${call.api} on ${call.lines} line(s) of ${call.file}`;
    if (!entry) problems.push(`${where}; review the call, then list it in DEPENDENCY_ALLOWED`);
    else if (entry.lines !== call.lines) problems.push(`${where}, but DEPENDENCY_ALLOWED allows ${entry.lines}`);
  }
  for (const entry of allowed) {
    if (crates.has(entry.crate) && !calls.some((c) => same(c, entry))) {
      problems.push(`${entry.crate} no longer names ${entry.api} in ${entry.file}; remove it from DEPENDENCY_ALLOWED`);
    }
  }
  return problems;
}

export function check(root) {
  const errors = [];
  const report = (file, line, message) =>
    errors.push({ file: relative(root, file).replaceAll('\\', '/'), line, message });

  for (const file of filesUnder(root, SKIP_PATHS)) {
    for (const hit of scanSource(readFileSync(file, 'utf8'))) {
      report(file, hit.line, `background capture: ${hit.name}`);
    }
  }

  const lib = join(root, 'src-tauri/src/lib.rs');
  if (!existsSync(lib)) report(lib, 1, 'src-tauri/src/lib.rs is missing');
  else if (!hasRequiredFilter(readFileSync(lib, 'utf8'))) {
    report(lib, 1, 'the Builder must set .device_event_filter(tauri::DeviceEventFilter::Always)');
  }

  const manifest = join(root, 'Cargo.toml');
  if (existsSync(manifest)) {
    for (const target of TARGETS) {
      const run = cargoTree(root, target);
      if (run.status !== 0) {
        report(manifest, 1, `cargo tree failed for ${target}: ${(run.stderr || String(run.error)).trim()}`);
      } else {
        for (const crate of scanCargoTree(run.stdout)) report(manifest, 1, `capture crate ${crate} on ${target}`);
      }
      const meta = cargoMetadata(root, target);
      if (meta.status !== 0) {
        report(manifest, 1, `cargo metadata failed for ${target}: ${(meta.stderr || String(meta.error)).trim()}`);
      } else {
        const packages = shippedPackages(JSON.parse(meta.stdout));
        const crates = new Set(packages.map((p) => p.name));
        for (const problem of unlistedDependencyCalls(scanDependencies(packages), crates)) {
          report(manifest, 1, `dependency capture on ${target}: ${problem}`);
        }
      }
    }
  }

  const lock = join(root, 'package-lock.json');
  if (existsSync(lock) && NPM_PLUGINS.test(readFileSync(lock, 'utf8'))) {
    report(lock, 1, 'plugin @tauri-apps/plugin-global-shortcut');
  }

  return errors;
}

// GitHub reads one workflow command per line.
const escape = (s) => String(s).replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A');

if (import.meta.main) {
  const root = resolve(process.argv[2] ?? '.');
  const errors = check(root);
  for (const e of errors) {
    console.error(
      process.env.GITHUB_ACTIONS
        ? `::error file=${e.file},line=${e.line}::${escape(e.message)}`
        : `${e.file}:${e.line}  ${e.message}`,
    );
  }
  if (errors.length) {
    console.error(`\nCapture check failed: ${errors.length} problem(s). keytriage reads keys only in a focused test.`);
    process.exit(1);
  }
  console.log('Capture check passed.');
}
