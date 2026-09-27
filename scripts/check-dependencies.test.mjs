// Positive controls for the dependency guard: an unlisted crate must make it fail.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { unlistedDependencies } from './check-dependencies.mjs';

const script = join(dirname(fileURLToPath(import.meta.url)), 'check-dependencies.mjs');

const pkg = (name, dependencies) => ({ name, manifest_path: `/${name}/Cargo.toml`, dependencies });
const dep = (name, kind = null) => ({ name, kind });

test('flags a direct dependency that is not listed, for normal and build dependencies', () => {
  const metadata = { packages: [pkg('app', [dep('tauri'), dep('multiinput'), dep('cc', 'build')])] };
  const found = unlistedDependencies(metadata, { app: ['tauri'] }).map((p) => p.dep);
  assert.deepEqual(found, ['multiinput', 'cc']);
});

test('allows listed crates, workspace members and dev-dependencies', () => {
  const metadata = {
    packages: [
      pkg('app', [dep('tauri'), dep('engine'), dep('proptest', 'dev')]),
      pkg('engine', []),
    ],
  };
  assert.deepEqual(unlistedDependencies(metadata, { app: ['tauri'] }), []);
});

test('a crate with no allowlist entry may not depend on anything', () => {
  const metadata = { packages: [pkg('new-crate', [dep('rdev')])] };
  assert.deepEqual(unlistedDependencies(metadata, {}).map((p) => p.dep), ['rdev']);
});

function crate(root, dir, name, extra = '') {
  mkdirSync(join(root, dir, 'src'), { recursive: true });
  writeFileSync(join(root, dir, 'Cargo.toml'), `[package]\nname = "${name}"\nversion = "0.0.0"\nedition = "2024"\n${extra}`);
  writeFileSync(join(root, dir, 'src/lib.rs'), '\n');
}

test('the CLI exits 1 on a planted unlisted crate and 0 without it', () => {
  const root = mkdtempSync(join(tmpdir(), 'keytriage-deps-'));
  try {
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/input"]\nexclude = ["vendor"]\nresolver = "3"\n');
    crate(root, 'crates/input', 'keytriage-input');
    crate(root, 'vendor/multiinput', 'multiinput');
    const run = () =>
      spawnSync(process.execPath, [script, root], { encoding: 'utf8', env: { ...process.env, GITHUB_ACTIONS: '' } });
    assert.equal(run().status, 0, run().stderr);

    crate(root, 'crates/input', 'keytriage-input', '\n[dependencies]\nmultiinput = { path = "../../vendor/multiinput" }\n');
    const result = run();
    assert.equal(result.status, 1);
    assert.match(result.stderr, /crates\/input\/Cargo\.toml:7 {2}keytriage-input depends on multiinput, which is not on its allowlist/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
