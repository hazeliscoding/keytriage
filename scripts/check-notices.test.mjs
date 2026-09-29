// Positive controls for the notices check: each gap in a notice must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  OVERLAY,
  fontProblems,
  linkedCrates,
  missingCrates,
  portableProblems,
  sectionProblems,
  tauriApiProblems,
  textProblems,
} from './check-notices.mjs';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const script = join(repo, 'scripts/check-notices.mjs');

// The template's fixed sections are the notices file's opening, so the controls cut them.
const template = readFileSync(join(repo, 'scripts/notices/about.hbs'), 'utf8').replaceAll('\r\n', '\n');
const fixed = template.slice(0, template.indexOf('{{#each licenses}}'));
const RULE = '='.repeat(80);
const withoutSection = (text, title) =>
  text
    .split(`${RULE}\n`)
    .filter((part) => !part.startsWith(`${title}\n`))
    .join(`${RULE}\n`);

const DASHES = '-'.repeat(80);
const npmSection = (pkg, license, body) => `${DASHES}\nPackage: ${pkg}\nLicense: "${license}"\n\n${body}\n`;
const APACHE = 'Apache License\nVersion 2.0, January 2004\n\nTERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION\n';
const OFL = 'Copyright 2015 The Authors\n\nSIL OPEN FONT LICENSE Version 1.1 - 26 February 2007\n';
const SPDX = 'SPDXVersion: SPDX-2.1\nDataLicense: CC0-1.0\nPackageName: tauri\nPackageLicenseDeclared: Apache-2.0\n';
const npmNotices = (sections) => `\n${sections.join('')}${DASHES}\n`;
const FONTS = ['@fontsource/barlow', '@fontsource/barlow-semi-condensed', '@fontsource/public-sans'];

test('the template holds every fixed section and no placeholder', () => {
  assert.deepEqual(sectionProblems(fixed), []);
  assert.deepEqual(textProblems(template), []);
});

test('reads linked crates from cargo tree, without workspace members or repeats', () => {
  const tree = [
    'keytriage v0.0.0 (C:\\code\\keytriage\\src-tauri)',
    'keytriage-input v0.0.0 (C:\\code\\keytriage\\crates\\input)',
    'windows v0.62.2',
    'windows-core v0.62.2',
    'windows-core v0.62.2 (*)',
    'vendored v0.1.0 (C:\\elsewhere\\vendored)',
  ].join('\r\n');
  assert.deepEqual(linkedCrates(tree, new Set(['keytriage', 'keytriage-input'])), [
    'vendored 0.1.0',
    'windows 0.62.2',
    'windows-core 0.62.2',
  ]);
});

test('flags a linked crate that has no notice', () => {
  const notices = `${fixed}- windows 0.62.2 (https://github.com/microsoft/windows-rs)\n`;
  assert.deepEqual(missingCrates(['windows 0.62.2'], notices), []);
  assert.deepEqual(missingCrates(['windows 0.62.2', 'windows-core 0.62.2'], notices), ['windows-core 0.62.2']);
  assert.deepEqual(missingCrates(['windows 0.62.3'], notices), ['windows 0.62.3']);
});

test("flags cargo-about's MIT and BSD placeholder copyright lines", () => {
  const mit = textProblems('MIT License\n\nCopyright (c) <year> <copyright holders>\n');
  const bsd = textProblems('Copyright (c) <year> <owner>. All rights reserved.\n');
  assert.equal(mit.length, 1);
  assert.equal(mit[0].line, 3);
  assert.match(mit[0].message, /placeholder/);
  assert.equal(bsd.length, 1);
  assert.equal(textProblems('Copyright (c) 2016 Dropbox, Inc.\n').length, 0);
});

test('flags an HTML entity in the plain-text notices', () => {
  const found = textProblems('BSD 3-Clause &quot;New&quot; or &quot;Revised&quot; License (BSD-3-Clause)\n');
  assert.equal(found.length, 1);
  assert.match(found[0].message, /&quot;/);
  assert.equal(textProblems('rights &amp; duties &#169; &#xA9;\n').length, 1);
  assert.equal(textProblems('BSD 3-Clause "New" or "Revised" License\n').length, 0);
});

test('flags a missing Microsoft loader or Rust standard library section', () => {
  const microsoft = sectionProblems(withoutSection(fixed, 'Microsoft WebView2 SDK loader'));
  assert.deepEqual(microsoft.map((p) => p.message), ['no "Microsoft WebView2 SDK loader" section']);
  const std = sectionProblems(withoutSection(fixed, 'The Rust standard library'));
  assert.deepEqual(std.map((p) => p.message), ['no "The Rust standard library" section']);
  for (const title of ['Microsoft Edge WebView2 Runtime bootstrapper', 'NSIS']) {
    assert.equal(sectionProblems(withoutSection(fixed, title)).length, 1, title);
  }
});

test("flags a Microsoft section without the license's binary clause", () => {
  const cut = fixed.replace('Redistributions in binary form must reproduce the above', 'Redistributions');
  assert.deepEqual(sectionProblems(cut).map((p) => p.message), [
    'the "Microsoft WebView2 SDK loader" section lacks "Redistributions in binary form must reproduce the above"',
  ]);
});

test('flags a shipped font whose package has no OFL notice', () => {
  const files = ['barlow-semi-condensed-latin-600-normal-YSX2SY2B.woff2', 'public-sans-latin-400-normal-SQG225MJ.woff2'];
  const complete = npmNotices([
    npmSection('@fontsource/public-sans', 'OFL-1.1', OFL),
    npmSection('@fontsource/barlow-semi-condensed', 'OFL-1.1', OFL),
  ]);
  assert.deepEqual(fontProblems(complete, files, FONTS), []);

  // The shorter family's notice doesn't cover the longer one.
  const shorter = npmNotices([
    npmSection('@fontsource/public-sans', 'OFL-1.1', OFL),
    npmSection('@fontsource/barlow', 'OFL-1.1', OFL),
  ]);
  assert.deepEqual(fontProblems(shorter, files, FONTS).map((p) => p.message), [
    'no notice for @fontsource/barlow-semi-condensed, whose font ships',
  ]);

  const noText = npmNotices([
    npmSection('@fontsource/public-sans', 'OFL-1.1', 'Copyright 2015 The Authors\n'),
    npmSection('@fontsource/barlow-semi-condensed', 'OFL-1.1', OFL),
  ]);
  assert.equal(fontProblems(noText, files, FONTS).length, 1);

  const stray = fontProblems(complete, [...files, 'inter-latin-400-normal-AAAA.woff2'], FONTS);
  assert.deepEqual(stray.map((p) => p.message), [
    'font inter-latin-400-normal-AAAA.woff2 comes from no @fontsource package in package.json',
  ]);
});

test('flags an @tauri-apps/api notice that holds only LICENSE.spdx', () => {
  assert.deepEqual(tauriApiProblems(npmNotices([npmSection('@tauri-apps/api', 'Apache-2.0 OR MIT', APACHE)])), []);
  const spdx = tauriApiProblems(npmNotices([npmSection('@tauri-apps/api', 'Apache-2.0 OR MIT', SPDX)]));
  assert.deepEqual(spdx.map((p) => p.message), ['the @tauri-apps/api notice holds no license text']);
  assert.equal(tauriApiProblems(npmNotices([npmSection('rxjs', 'Apache-2.0', APACHE)])).length, 1);
});

test("flags a portable folder that doesn't ship the installer's license files byte for byte", () => {
  const { resources } = JSON.parse(readFileSync(join(repo, OVERLAY), 'utf8')).bundle;
  const text = (source) => Buffer.from(`text of ${source}\n`);
  const sources = new Map(Object.keys(resources).map((s) => [s, text(s)]));
  const complete = () => new Map(Object.entries(resources).map(([s, t]) => [t, text(s)]));
  assert.deepEqual(portableProblems(resources, sources, complete()), []);

  const [[source, target]] = Object.entries(resources);
  const missing = complete();
  missing.delete(target);
  assert.deepEqual(portableProblems(resources, sources, missing), [
    { file: target, message: `missing; the installer ships it from ${source}` },
  ]);

  const stale = complete();
  stale.set(target, Buffer.from(`text of ${source}\r\n`));
  assert.deepEqual(portableProblems(resources, sources, stale).map((p) => p.message), [
    `differs from ${source}, which the installer ships`,
  ]);

  const unbuilt = new Map(sources);
  unbuilt.delete(source);
  assert.deepEqual(portableProblems(resources, unbuilt, complete()).map((p) => p.message), [
    `can't be compared, because ${source} is missing`,
  ]);

  const extra = complete();
  extra.set('licenses/NOTICE.txt', Buffer.from('x'));
  assert.deepEqual(portableProblems(resources, sources, extra), [
    { file: 'licenses/NOTICE.txt', message: 'is not a license file the installer ships' },
  ]);
});

function crate(root, dir, name, extra = '') {
  mkdirSync(join(root, dir, 'src'), { recursive: true });
  writeFileSync(join(root, dir, 'Cargo.toml'), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${extra}`);
  writeFileSync(join(root, dir, 'src/lib.rs'), '\n');
}

test('the CLI exits 1 on a missing crate or a planted placeholder, and 0 when complete', () => {
  const root = mkdtempSync(join(tmpdir(), 'keytriage-notices-'));
  try {
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["src-tauri"]\nexclude = ["vendor"]\nresolver = "3"\n');
    crate(root, 'src-tauri', 'keytriage', '\n[dependencies]\nlinked = { path = "../vendor/linked" }\n');
    crate(root, 'vendor/linked', 'linked');
    const lock = spawnSync('cargo', ['generate-lockfile', '--offline'], { cwd: root, encoding: 'utf8' });
    assert.equal(lock.status, 0, lock.stderr);

    writeFileSync(join(root, 'package.json'), JSON.stringify({ dependencies: { '@fontsource/public-sans': '5.3.0' } }));
    mkdirSync(join(root, 'dist/keytriage/browser/media'), { recursive: true });
    writeFileSync(join(root, 'dist/keytriage/browser/media/public-sans-latin-400-normal-SQG225MJ.woff2'), '');
    writeFileSync(
      join(root, 'dist/keytriage/3rdpartylicenses.txt'),
      npmNotices([
        npmSection('@tauri-apps/api', 'Apache-2.0 OR MIT', APACHE),
        npmSection('@fontsource/public-sans', 'OFL-1.1', OFL),
      ]),
    );
    mkdirSync(join(root, 'target/notices'), { recursive: true });
    const notices = join(root, 'target/notices/THIRD-PARTY-RUST.txt');
    const run = (...args) =>
      spawnSync(process.execPath, [script, ...args, root], { encoding: 'utf8', env: { ...process.env, GITHUB_ACTIONS: '' } });

    writeFileSync(notices, fixed);
    const missing = run();
    assert.equal(missing.status, 1, missing.stderr);
    assert.match(missing.stderr, /THIRD-PARTY-RUST\.txt:1 {2}linked 0\.1\.0 is linked into keytriage\.exe but has no notice/);

    writeFileSync(notices, `${fixed}- linked 0.1.0 (https://crates.io/crates/linked)\n\nCopyright (c) 2026 Someone\n`);
    const clean = run();
    assert.equal(clean.status, 0, clean.stderr);

    writeFileSync(join(root, 'LICENSE'), 'Apache License\n');
    const resources = { '../LICENSE': 'licenses/LICENSE.txt', '../target/notices/THIRD-PARTY-RUST.txt': 'licenses/THIRD-PARTY-RUST.txt' };
    writeFileSync(join(root, OVERLAY), JSON.stringify({ bundle: { resources } }));
    const portable = join(root, 'target/portable/keytriage');
    mkdirSync(join(portable, 'licenses'), { recursive: true });
    writeFileSync(join(portable, 'licenses/LICENSE.txt'), 'Apache License\n');
    const unstaged = run('--portable', portable);
    assert.equal(unstaged.status, 1);
    assert.match(
      unstaged.stderr,
      /target\/portable\/keytriage\/licenses\/THIRD-PARTY-RUST\.txt:1 {2}missing; the installer ships it from \.\.\/target\/notices\/THIRD-PARTY-RUST\.txt/,
    );
    writeFileSync(join(portable, 'licenses/THIRD-PARTY-RUST.txt'), readFileSync(notices));
    const staged = run('--portable', portable);
    assert.equal(staged.status, 0, staged.stderr);

    writeFileSync(notices, `${fixed}- linked 0.1.0 (https://crates.io/crates/linked)\n\nCopyright (c) <year> <owner>\n`);
    const planted = run();
    assert.equal(planted.status, 1);
    assert.match(planted.stderr, /THIRD-PARTY-RUST\.txt:\d+ {2}placeholder <year>/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
