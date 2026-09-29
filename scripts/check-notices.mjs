// Notices check: the installer ships THIRD-PARTY-RUST.txt, which cargo-about writes from
// scripts/notices, and Angular's 3rdpartylicenses.txt as THIRD-PARTY-NPM.txt. This fails when
// either leaves out code that ships, or prints a placeholder where a copyright line belongs.
//
// Run it after `npm run build` and the cargo-about generate command in AGENTS.md.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';

export const RUST_NOTICES = 'target/notices/THIRD-PARTY-RUST.txt';
export const NPM_NOTICES = 'dist/keytriage/3rdpartylicenses.txt';
export const FONT_MEDIA = 'dist/keytriage/browser/media';
export const TARGET = 'x86_64-pc-windows-msvc';

// cargo-about prints these when a crate ships no file with its copyright line, and Handlebars'
// double braces print entities into a plain-text file.
const PLACEHOLDER = /<year>|<owner>|<copyright holders>/;
const ENTITY = /&(?:[a-z]+|#\d+|#x[0-9a-f]+);/i;

// The template's own sections, for code that no crate's license covers. Each must hold its text.
export const FIXED_SECTIONS = [
  { title: 'The Rust standard library', needs: ['MIT OR Apache-2.0'] },
  {
    title: 'Microsoft WebView2 SDK loader',
    needs: [
      'Copyright (C) Microsoft Corporation. All rights reserved.',
      'Redistributions in binary form must reproduce the above',
    ],
  },
  { title: 'Microsoft Edge WebView2 Runtime bootstrapper', needs: ['MicrosoftEdgeWebview2Setup.exe'] },
  { title: 'NSIS', needs: ['zlib/libpng'] },
];

const SECTION_RULE = /^={80}$/;
const NPM_RULE = /^-{80}$/;
const FONT_EXTENSIONS = new Set(['.woff', '.woff2', '.ttf', '.otf', '.eot']);
const OFL_TEXT = 'SIL OPEN FONT LICENSE Version 1.1';
const LICENSE_TEXTS = [
  'TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION',
  'Permission is hereby granted, free of charge',
];

// `cargo tree --prefix none` prints "name vX.Y.Z", then the source for anything outside crates.io
// and " (*)" on a repeat, so only the first two words count.
export function linkedCrates(tree, members) {
  const crates = new Set();
  for (const line of tree.split(/\r?\n/)) {
    const match = /^(\S+) v(\S+)/.exec(line.trim());
    if (match && !members.has(match[1])) crates.add(`${match[1]} ${match[2]}`);
  }
  return [...crates].sort();
}

export function listedCrates(notices) {
  const listed = new Set();
  for (const line of notices.split(/\r?\n/)) {
    const match = /^- (\S+) (\S+) \(/.exec(line);
    if (match) listed.add(`${match[1]} ${match[2]}`);
  }
  return listed;
}

export function missingCrates(linked, notices) {
  const listed = listedCrates(notices);
  return linked.filter((c) => !listed.has(c));
}

export function textProblems(text) {
  const problems = [];
  text.split(/\r?\n/).forEach((line, i) => {
    const placeholder = PLACEHOLDER.exec(line);
    if (placeholder) {
      problems.push({ line: i + 1, message: `placeholder ${placeholder[0]} in place of a copyright line` });
    }
    const entity = ENTITY.exec(line);
    if (entity) problems.push({ line: i + 1, message: `HTML entity ${entity[0]} in a plain-text file` });
  });
  return problems;
}

function sections(text, rule) {
  const found = [];
  let current = null;
  text.split(/\r?\n/).forEach((line, i) => {
    if (rule.test(line)) {
      current = { line: i + 2, lines: [] };
      found.push(current);
    } else if (current) {
      current.lines.push(line);
    }
  });
  return found;
}

export function sectionProblems(notices) {
  const found = sections(notices, SECTION_RULE);
  const problems = [];
  for (const { title, needs } of FIXED_SECTIONS) {
    const section = found.find((s) => s.lines[0] === title);
    if (!section) {
      problems.push({ line: 1, message: `no "${title}" section` });
      continue;
    }
    const body = section.lines.join('\n');
    for (const need of needs.filter((n) => !body.includes(n))) {
      problems.push({ line: section.line, message: `the "${title}" section lacks "${need}"` });
    }
  }
  return problems;
}

// Angular's notices: sections between rules of 80 dashes, each opening with Package and License.
export function npmSections(text) {
  return sections(text, NPM_RULE).flatMap(({ line, lines }) => {
    const pkg = /^Package: (.+)$/.exec(lines[0] ?? '');
    if (!pkg) return [];
    const license = /^License: "?(.*?)"?$/.exec(lines[1] ?? '');
    return [{ line, pkg: pkg[1], license: license?.[1] ?? '', body: lines.slice(2).join('\n') }];
  });
}

// Each font file is matched to the @fontsource package whose name starts its file name, the
// longest first, so a family never borrows a longer family's notice.
export function fontProblems(npmText, fontFiles, fontPackages) {
  const byLength = [...fontPackages].sort((a, b) => b.length - a.length);
  const used = new Set();
  const problems = [];
  for (const file of fontFiles) {
    const pkg = byLength.find((p) => file.startsWith(`${p.slice('@fontsource/'.length)}-`));
    if (pkg) used.add(pkg);
    else problems.push({ line: 1, message: `font ${file} comes from no @fontsource package in package.json` });
  }
  const found = npmSections(npmText);
  for (const pkg of [...used].sort()) {
    const section = found.find((s) => s.pkg === pkg);
    if (!section) problems.push({ line: 1, message: `no notice for ${pkg}, whose font ships` });
    else if (section.license !== 'OFL-1.1' || !section.body.includes(OFL_TEXT)) {
      problems.push({ line: section.line, message: `the ${pkg} notice lacks OFL-1.1 or the OFL text` });
    }
  }
  return problems;
}

// Angular copies only the first LICENSE* file it lists, and @tauri-apps/api also ships
// LICENSE.spdx, which names licenses without their text.
export function tauriApiProblems(npmText) {
  const section = npmSections(npmText).find((s) => s.pkg === '@tauri-apps/api');
  if (!section) return [{ line: 1, message: 'no notice for @tauri-apps/api' }];
  if (LICENSE_TEXTS.some((t) => section.body.includes(t))) return [];
  return [{ line: section.line, message: 'the @tauri-apps/api notice holds no license text' }];
}

function cargo(root, args) {
  return spawnSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
}

export function check(root) {
  const errors = [];
  const report = (file, line, message) => errors.push({ file, line, message });
  const read = (file, hint) => {
    const path = join(root, file);
    if (existsSync(path)) return readFileSync(path, 'utf8');
    report(file, 1, `missing; ${hint}`);
    return null;
  };

  const rust = read(RUST_NOTICES, 'run the cargo about generate command in AGENTS.md first');
  if (rust !== null) {
    for (const p of [...textProblems(rust), ...sectionProblems(rust)]) report(RUST_NOTICES, p.line, p.message);
    const metadata = cargo(root, ['metadata', '--format-version', '1', '--no-deps', '--locked']);
    const tree = cargo(root, [
      'tree', '-p', 'keytriage', '--target', TARGET, '-e', 'normal,no-proc-macro', '--prefix', 'none', '--locked',
    ]);
    if (metadata.status !== 0 || tree.status !== 0) {
      report('Cargo.toml', 1, `cargo failed: ${(metadata.stderr || tree.stderr || '').trim()}`);
    } else {
      const members = new Set(JSON.parse(metadata.stdout).packages.map((p) => p.name));
      for (const c of missingCrates(linkedCrates(tree.stdout, members), rust)) {
        report(RUST_NOTICES, 1, `${c} is linked into keytriage.exe but has no notice`);
      }
    }
  }

  const npm = read(NPM_NOTICES, 'run npm run build first');
  if (npm !== null) {
    for (const p of textProblems(npm)) report(NPM_NOTICES, p.line, p.message);
    const media = join(root, FONT_MEDIA);
    const fonts = existsSync(media) ? readdirSync(media).filter((f) => FONT_EXTENSIONS.has(extname(f))) : [];
    const manifest = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
    const fontPackages = Object.keys(manifest.dependencies ?? {}).filter((d) => d.startsWith('@fontsource/'));
    for (const p of [...fontProblems(npm, fonts, fontPackages), ...tauriApiProblems(npm)]) {
      report(NPM_NOTICES, p.line, p.message);
    }
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
    console.error(
      `\nNotices check failed: ${errors.length} problem(s). Every shipped component needs its full notice.`,
    );
    process.exit(1);
  }
  console.log('Notices check passed.');
}
