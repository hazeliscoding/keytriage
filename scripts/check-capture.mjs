// Capture guard: fails when code could read keys outside a focused test window.
// It enforces the "foreground capture only" line of the privacy contract in README.md.
//
// Limits: it matches names, not behavior. A dependency that captures under a name not listed here,
// code generated at build time and deliberate obfuscation get past it. Review covers those.
// GetKeyState and GetKeyboardState are allowed: they read the calling thread's own input state.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';

const BANNED = [
  { pattern: /\bRIDEV_INPUTSINK\b/, name: 'RIDEV_INPUTSINK' },
  { pattern: /\bRIDEV_EXINPUTSINK\b/, name: 'RIDEV_EXINPUTSINK' },
  { pattern: /\bSetWindowsHookEx[AW]?\b/, name: 'SetWindowsHookEx' },
  { pattern: /\bWH_KEYBOARD(_LL)?\b/, name: 'WH_KEYBOARD_LL' },
  // These read keys while another app has focus.
  { pattern: /\bGetAsyncKeyState\b/, name: 'GetAsyncKeyState' },
  { pattern: /\bRegisterHotKey\b/, name: 'RegisterHotKey' },
  { pattern: /\bDirectInput8Create\b/, name: 'DirectInput8Create' },
  { pattern: /\bDISCL_BACKGROUND\b/, name: 'DISCL_BACKGROUND' },
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

const NPM_PLUGINS = /node_modules\/@tauri-apps\/plugin-global-shortcut"/;

export const TARGETS = ['x86_64-pc-windows-msvc'];

const NATIVE_EXTS = [
  '.rs', '.c', '.cc', '.cpp', '.cxx', '.c++', '.h', '.hh', '.hpp', '.hxx', '.inl', '.ixx',
  '.asm', '.s',
];
// Build output and installed packages, by path: a folder with the same name elsewhere can hold
// source.
const SKIP_PATHS = new Set(['.angular', 'dist', 'node_modules', 'src-tauri/gen', 'target']);

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

export function scanSource(text) {
  const hits = [];
  // Comments that explain the privacy contract name the APIs it bans. Blank them in place so
  // offsets and line numbers stay the same.
  const code = text.replace(/^[ \t]*\/\/.*$/gm, (c) => ' '.repeat(c.length));
  code.split('\n').forEach((line, i) => {
    for (const rule of BANNED) {
      if (rule.pattern.test(line)) hits.push({ line: i + 1, name: rule.name });
    }
  });
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
