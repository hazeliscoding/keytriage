// Positive controls for the release check: each drift between the versions, the tag, the release
// config and the staged release files must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  FILES,
  SUMS,
  artifactProblems,
  configProblems,
  lockMembers,
  overlayProblems,
  releaseFiles,
  versionProblems,
  workspaceVersion,
} from './check-release.mjs';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const script = join(repo, 'scripts/check-release.mjs');
const texts = Object.fromEntries(
  Object.entries(FILES).map(([k, f]) => [k, readFileSync(join(repo, f), 'utf8')]),
);
const version = JSON.parse(texts.packageJson).version;
const OTHER = '9.9.9';

const messages = (problems) => problems.map((p) => `${p.file}: ${p.message}`);
const edit = (text, from, to) => {
  assert.ok(text.includes(from), `fixture lacks ${from}`);
  return text.replace(from, to);
};
const json = (value) => `${JSON.stringify(value, null, 2)}\n`;
const base = () => JSON.parse(texts.base);
const overlay = () => JSON.parse(texts.overlay);

test("the repo's own files pass", () => {
  assert.deepEqual(versionProblems(texts), []);
  assert.deepEqual(versionProblems(texts, `v${version}`), []);
  assert.deepEqual(configProblems(texts.base), []);
  assert.deepEqual(overlayProblems(texts.overlay), []);
});

test('reads the workspace version and the members from Cargo.toml and Cargo.lock', () => {
  assert.equal(workspaceVersion(texts.cargoToml).version, version);
  assert.deepEqual(
    lockMembers(texts.cargoLock)
      .map((m) => m.name)
      .sort(),
    ['keytriage', 'keytriage-diagnostics', 'keytriage-input'],
  );
  const lock =
    '[[package]]\r\nname = "a"\r\nversion = "1.0.0"\r\nsource = "registry+x"\r\n\r\n[[package]]\r\nname = "b"\r\nversion = "2.0.0"\r\n';
  assert.deepEqual(lockMembers(lock), [{ line: 8, name: 'b', version: '2.0.0' }]);
  assert.equal(
    workspaceVersion('[package]\nversion = "1.0.0"\n[workspace.package]\nedition = "2024"\n'),
    null,
  );
});

test('flags a tag that does not name the version', () => {
  assert.deepEqual(messages(versionProblems(texts, `v${OTHER}`)), [
    `package.json: tag v${OTHER} is not v${version}`,
  ]);
  assert.equal(versionProblems(texts, version).length, 1);
});

test('flags a stale Cargo.lock member', () => {
  const cargoLock = edit(
    texts.cargoLock,
    `name = "keytriage-input"\nversion = "${version}"`,
    `name = "keytriage-input"\nversion = "${OTHER}"`,
  );
  const found = versionProblems({ ...texts, cargoLock });
  assert.deepEqual(messages(found), [
    `Cargo.lock: keytriage-input ${OTHER} is not ${version}; run cargo update --workspace`,
  ]);
  assert.equal(texts.cargoLock.split('\n')[found[0].line - 1], `version = "${version}"`);
  const empty = '[[package]]\nname = "a"\nversion = "1.0.0"\nsource = "registry+x"\n';
  assert.deepEqual(messages(versionProblems({ ...texts, cargoLock: empty })), [
    'Cargo.lock: Cargo.lock lists none of the workspace crates',
  ]);
});

test('flags a stale Cargo.toml workspace version', () => {
  const cargoToml = edit(texts.cargoToml, `version = "${version}"`, `version = "${OTHER}"`);
  assert.deepEqual(messages(versionProblems({ ...texts, cargoToml })), [
    `Cargo.toml: [workspace.package] version ${OTHER} is not ${version}`,
  ]);
});

test('flags either stale package-lock.json field', () => {
  const lock = JSON.parse(texts.packageLock);
  const top = json({ ...lock, version: OTHER });
  assert.deepEqual(messages(versionProblems({ ...texts, packageLock: top })), [
    `package-lock.json: version ${OTHER} is not ${version}`,
  ]);
  const root = json({
    ...lock,
    packages: { ...lock.packages, '': { ...lock.packages[''], version: OTHER } },
  });
  const found = versionProblems({ ...texts, packageLock: root });
  assert.deepEqual(messages(found), [
    `package-lock.json: packages[""].version ${OTHER} is not ${version}`,
  ]);
  assert.match(root.split('\n')[found[0].line - 1], new RegExp(`"version": "${OTHER}"`));
});

test('flags a missing or default publisher', () => {
  const missing = base();
  delete missing.bundle.publisher;
  assert.deepEqual(messages(configProblems(json(missing))), [
    'src-tauri/tauri.conf.json: bundle.publisher is not set',
  ]);
  const fallback = base();
  fallback.bundle.publisher = fallback.identifier.split('.')[1];
  assert.deepEqual(messages(configProblems(json(fallback))), [
    `src-tauri/tauri.conf.json: bundle.publisher is the identifier's default "${fallback.bundle.publisher}"`,
  ]);
});

test('flags a missing webviewInstallMode', () => {
  const config = base();
  delete config.bundle.windows.webviewInstallMode;
  assert.deepEqual(messages(configProblems(json(config))), [
    'src-tauri/tauri.conf.json: bundle.windows.webviewInstallMode is not set',
  ]);
  delete config.bundle.windows;
  assert.equal(configProblems(json(config)).length, 1);
});

test('flags resources in the base config and a version not taken from package.json', () => {
  const config = base();
  config.bundle.resources = { '../LICENSE': 'licenses/LICENSE.txt' };
  config.version = version;
  assert.deepEqual(messages(configProblems(json(config))), [
    'src-tauri/tauri.conf.json: version must come from "../package.json"',
    'src-tauri/tauri.conf.json: bundle.resources belongs in tauri.release.conf.json, which only the release build merges',
  ]);
});

test('flags an overlay that sets the CSP or plugins', () => {
  const csp = overlay();
  csp.app = { security: { csp: "default-src 'self'; connect-src *" } };
  assert.deepEqual(messages(overlayProblems(json(csp))), [
    'src-tauri/tauri.release.conf.json: "app" is outside bundle.resources',
  ]);
  const plugins = overlay();
  plugins.plugins = { updater: {} };
  assert.deepEqual(messages(overlayProblems(json(plugins))), [
    'src-tauri/tauri.release.conf.json: "plugins" is outside bundle.resources',
  ]);
  const other = overlay();
  other.bundle.publisher = 'someone';
  assert.equal(overlayProblems(json(other)).length, 1);
});

test('flags an overlay resource that lands outside licenses/', () => {
  const outside = overlay();
  outside.bundle.resources['../dist/keytriage/browser/'] = 'www/';
  outside.bundle.resources['../LICENSE'] = 'licenses/../LICENSE.txt';
  assert.deepEqual(messages(overlayProblems(json(outside))), [
    'src-tauri/tauri.release.conf.json: ../LICENSE lands at licenses/../LICENSE.txt, outside licenses/',
    'src-tauri/tauri.release.conf.json: ../dist/keytriage/browser/ lands at www/, outside licenses/',
  ]);
  const list = overlay();
  list.bundle.resources = ['licenses/LICENSE.txt'];
  assert.deepEqual(messages(overlayProblems(json(list))), [
    'src-tauri/tauri.release.conf.json: bundle.resources must map each license file to its place under licenses/',
  ]);
});

test('the CLI exits 0 on the repo and 1 on a tag that does not match', () => {
  const run = (...args) =>
    spawnSync(process.execPath, [script, ...args, repo], {
      encoding: 'utf8',
      env: { ...process.env, GITHUB_ACTIONS: '' },
    });
  const clean = run('--tag', `v${version}`);
  assert.equal(clean.status, 0, clean.stderr);
  const wrong = run('--tag', `v${OTHER}`);
  assert.equal(wrong.status, 1);
  assert.match(
    wrong.stderr,
    new RegExp(`package\\.json:\\d+ {2}tag v${OTHER.replaceAll('.', '\\.')} is not v`),
  );
});

const [ZIP, INSTALLER] = releaseFiles(version);
const NAMES = [ZIP, INSTALLER, SUMS];
const sha = (text) => createHash('sha256').update(text).digest('hex');
const sumLine = (name, mode = '*') => `${sha(name)} ${mode}${name}\n`;
const SUMS_TEXT = sumLine(ZIP) + sumLine(INSTALLER);
const artifact = (names, sums) => messages(artifactProblems(version, names, sums, sha));

test('names the zip first, then the installer', () => {
  assert.deepEqual(releaseFiles('1.2.3'), [
    'keytriage_1.2.3_x64-portable.zip',
    'keytriage_1.2.3_x64-setup.exe',
  ]);
});

test("the release files pass in either of sha256sum's modes", () => {
  assert.deepEqual(artifact(NAMES, SUMS_TEXT), []);
  assert.deepEqual(artifact(NAMES, sumLine(ZIP, ' ') + sumLine(INSTALLER, ' ')), []);
});

test('flags a missing zip, a missing SHA256SUMS.txt and a stray file', () => {
  assert.deepEqual(artifact([INSTALLER, SUMS, 'keytriage.exe'], SUMS_TEXT), [
    `${ZIP}: is missing from the release files`,
    'keytriage.exe: is not a release file',
  ]);
  assert.deepEqual(artifact([ZIP, INSTALLER], null), [
    `${SUMS}: is missing from the release files`,
  ]);
});

test('flags checksums out of order, for another version, missing or wrong', () => {
  assert.deepEqual(artifact(NAMES, sumLine(INSTALLER) + sumLine(ZIP)), [
    `${SUMS}: line 1 names ${INSTALLER}, not ${ZIP}`,
    `${SUMS}: line 2 names ${ZIP}, not ${INSTALLER}`,
  ]);
  const [otherZip] = releaseFiles(OTHER);
  assert.deepEqual(artifact(NAMES, sumLine(otherZip) + sumLine(INSTALLER)), [
    `${SUMS}: line 1 names ${otherZip}, not ${ZIP}`,
  ]);
  assert.deepEqual(artifact(NAMES, sumLine(ZIP)), [`${SUMS}: lists 1 files, not 2`]);
  const zero = '0'.repeat(64);
  assert.deepEqual(artifact(NAMES, `${zero} *${ZIP}\n${sumLine(INSTALLER)}`), [
    `${SUMS}: ${ZIP} does not hash to ${zero}`,
  ]);
  assert.deepEqual(artifact(NAMES, `${sha(ZIP)}  *${ZIP}\n${sumLine(INSTALLER)}`), [
    `${SUMS}: line 1 names *${ZIP}, not ${ZIP}`,
  ]);
});

test('flags CR line endings and a missing final line break', () => {
  assert.deepEqual(artifact(NAMES, SUMS_TEXT.replaceAll('\n', '\r\n')), [
    `${SUMS}: has CR line endings`,
  ]);
  assert.deepEqual(artifact(NAMES, SUMS_TEXT.trimEnd()), [
    `${SUMS}: does not end with a line break`,
  ]);
});

test('the CLI hashes a staged release folder', () => {
  const dir = mkdtempSync(join(tmpdir(), 'keytriage-release-'));
  try {
    writeFileSync(join(dir, ZIP), 'zip');
    writeFileSync(join(dir, INSTALLER), 'installer');
    writeFileSync(join(dir, SUMS), `${sha('zip')} *${ZIP}\n${sha('installer')} *${INSTALLER}\n`);
    const run = () =>
      spawnSync(process.execPath, [script, '--artifact', dir, repo], {
        encoding: 'utf8',
        env: { ...process.env, GITHUB_ACTIONS: '' },
      });
    const clean = run();
    assert.equal(clean.status, 0, clean.stderr);
    writeFileSync(join(dir, ZIP), 'another zip');
    const tampered = run();
    assert.equal(tampered.status, 1);
    assert.match(
      tampered.stderr,
      new RegExp(`SHA256SUMS\\.txt:1 {2}${ZIP.replaceAll('.', '\\.')} does not hash to`),
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
