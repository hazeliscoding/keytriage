// Release check: the version lives in package.json, package-lock.json, Cargo.toml and Cargo.lock,
// and a release tag must name it. The Tauri config keeps the installer's publisher and WebView2
// mode explicit, and the release overlay adds license files and nothing else. check-network.mjs
// reads only tauri.conf.json, so an overlay that loosened the CSP or added a plugin would ship
// unchecked.
//
// Usage: node scripts/check-release.mjs [--tag vX.Y.Z] [root]
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

export const FILES = {
  packageJson: 'package.json',
  packageLock: 'package-lock.json',
  cargoToml: 'Cargo.toml',
  cargoLock: 'Cargo.lock',
  base: 'src-tauri/tauri.conf.json',
  overlay: 'src-tauri/tauri.release.conf.json',
};

// The first line at or after `from` that matches, or 1.
function lineOf(text, pattern, from = 0) {
  const lines = text.split(/\r?\n/);
  for (let i = from; i < lines.length; i++) if (pattern.test(lines[i])) return i + 1;
  return 1;
}

export function workspaceVersion(cargoToml) {
  let inSection = false;
  const lines = cargoToml.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.startsWith('[')) {
      inSection = line === '[workspace.package]';
      continue;
    }
    const match = inSection && /^version\s*=\s*"([^"]*)"/.exec(line);
    if (match) return { version: match[1], line: i + 1 };
  }
  return null;
}

// Cargo.lock gives no source to the workspace's own crates, which is how they are told apart.
export function lockMembers(cargoLock) {
  const entries = [];
  let entry = null;
  cargoLock.split(/\r?\n/).forEach((line, i) => {
    if (line === '[[package]]') {
      entry = { line: i + 1 };
      entries.push(entry);
    } else if (line.startsWith('[')) {
      entry = null;
    } else if (entry) {
      const field = /^(name|version|source) = "(.*)"$/.exec(line);
      if (field) entry[field[1]] = field[2];
      if (field?.[1] === 'version') entry.line = i + 1;
    }
  });
  return entries.filter((e) => e.name && e.source === undefined);
}

export function versionProblems(texts, tag) {
  const problems = [];
  const report = (file, line, message) => problems.push({ file, line, message });
  const version = JSON.parse(texts.packageJson).version;
  if (typeof version !== 'string' || version === '') {
    report(FILES.packageJson, 1, 'package.json has no version');
    return problems;
  }

  const lock = JSON.parse(texts.packageLock);
  if (lock.version !== version) {
    report(
      FILES.packageLock,
      lineOf(texts.packageLock, /^ {2}"version":/),
      `version ${lock.version} is not ${version}`,
    );
  }
  const root = lock.packages?.[''];
  if (root?.version !== version) {
    const at = lineOf(texts.packageLock, /^ {4}"": \{/);
    report(
      FILES.packageLock,
      lineOf(texts.packageLock, /"version":/, at),
      `packages[""].version ${root?.version} is not ${version}`,
    );
  }

  const workspace = workspaceVersion(texts.cargoToml);
  if (!workspace) report(FILES.cargoToml, 1, '[workspace.package] has no version');
  else if (workspace.version !== version) {
    report(
      FILES.cargoToml,
      workspace.line,
      `[workspace.package] version ${workspace.version} is not ${version}`,
    );
  }

  const members = lockMembers(texts.cargoLock);
  if (members.length === 0) {
    report(FILES.cargoLock, 1, 'Cargo.lock lists none of the workspace crates');
  }
  for (const m of members.filter((m) => m.version !== version)) {
    report(
      FILES.cargoLock,
      m.line,
      `${m.name} ${m.version} is not ${version}; run cargo update --workspace`,
    );
  }

  if (tag !== undefined && tag !== `v${version}`) {
    report(
      FILES.packageJson,
      lineOf(texts.packageJson, /"version":/),
      `tag ${tag} is not v${version}`,
    );
  }
  return problems;
}

export function configProblems(baseText) {
  const problems = [];
  const report = (pattern, message) =>
    problems.push({ file: FILES.base, line: lineOf(baseText, pattern), message });
  const config = JSON.parse(baseText);
  const bundle = config.bundle ?? {};

  if (config.version !== '../package.json') {
    report(/"version":/, 'version must come from "../package.json"');
  }
  // Without a publisher, Tauri takes the identifier's second part, and NSIS keys the remembered
  // install folder on it.
  const fallback = String(config.identifier ?? '').split('.')[1];
  if (typeof bundle.publisher !== 'string' || bundle.publisher.trim() === '') {
    report(/"bundle":/, 'bundle.publisher is not set');
  } else if (bundle.publisher === fallback) {
    report(/"publisher":/, `bundle.publisher is the identifier's default "${fallback}"`);
  }
  if (typeof bundle.windows?.webviewInstallMode?.type !== 'string') {
    report(/"bundle":/, 'bundle.windows.webviewInstallMode is not set');
  }
  // tauri-build copies resources on every cargo build and fails on a missing file.
  if ('resources' in bundle) {
    report(
      /"resources":/,
      'bundle.resources belongs in tauri.release.conf.json, which only the release build merges',
    );
  }
  return problems;
}

const LICENSE_TARGET = /^licenses\/[^/\\]+$/;

export function overlayProblems(overlayText) {
  const problems = [];
  const report = (pattern, message) =>
    problems.push({ file: FILES.overlay, line: lineOf(overlayText, pattern), message });
  const overlay = JSON.parse(overlayText);
  const literal = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  for (const key of Object.keys(overlay).filter((k) => k !== 'bundle')) {
    report(new RegExp(`^ {2}"${literal(key)}":`), `"${key}" is outside bundle.resources`);
  }
  const bundle = overlay.bundle ?? {};
  for (const key of Object.keys(bundle).filter((k) => k !== 'resources')) {
    report(new RegExp(`"${literal(key)}":`), `bundle.${key} is outside bundle.resources`);
  }
  const resources = bundle.resources;
  // The list form puts ../ paths under _up_, so only a map names where each file lands.
  if (resources === null || typeof resources !== 'object' || Array.isArray(resources)) {
    report(
      /"resources":/,
      'bundle.resources must map each license file to its place under licenses/',
    );
    return problems;
  }
  for (const [source, target] of Object.entries(resources)) {
    if (!LICENSE_TARGET.test(target)) {
      report(
        new RegExp(`"${literal(source)}":`),
        `${source} lands at ${target}, outside licenses/`,
      );
    }
  }
  return problems;
}

export function check(root, tag) {
  const texts = Object.fromEntries(
    Object.entries(FILES).map(([k, f]) => [k, readFileSync(join(root, f), 'utf8')]),
  );
  return [
    ...versionProblems(texts, tag),
    ...configProblems(texts.base),
    ...overlayProblems(texts.overlay),
  ];
}

// GitHub reads one workflow command per line.
const escape = (s) =>
  String(s).replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A');

if (import.meta.main) {
  const { values, positionals } = parseArgs({
    options: { tag: { type: 'string' } },
    allowPositionals: true,
  });
  const errors = check(resolve(positionals[0] ?? '.'), values.tag);
  for (const e of errors) {
    console.error(
      process.env.GITHUB_ACTIONS
        ? `::error file=${e.file},line=${e.line}::${escape(e.message)}`
        : `${e.file}:${e.line}  ${e.message}`,
    );
  }
  if (errors.length) {
    console.error(
      `\nRelease check failed: ${errors.length} problem(s). A version bump changes package.json, package-lock.json, Cargo.toml and Cargo.lock together.`,
    );
    process.exit(1);
  }
  console.log('Release check passed.');
}
