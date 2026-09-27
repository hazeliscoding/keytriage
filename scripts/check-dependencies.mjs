// Dependency guard: fails when a workspace crate gains a direct dependency that is not on its
// allowlist. The network and capture guards match known names, so a crate that connects or
// captures under an unlisted name would get past them. This makes each new crate a reviewed choice.
//
// To add a dependency, first check that it neither captures keys outside a focused test nor opens
// network connections, then list it below in the same change. Dev-dependencies don't ship and are
// not checked. Transitive crates are left to the network and capture guards.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';

export const ALLOWED = {
  keytriage: ['tauri', 'tauri-build'],
  'keytriage-input': [],
  'keytriage-diagnostics': [],
};

export function unlistedDependencies(metadata, allowed = ALLOWED) {
  const members = new Set(metadata.packages.map((p) => p.name));
  const problems = [];
  for (const pkg of metadata.packages) {
    const listed = new Set(allowed[pkg.name] ?? []);
    for (const dep of pkg.dependencies) {
      if (dep.kind === 'dev' || members.has(dep.name) || listed.has(dep.name)) continue;
      problems.push({ manifest: pkg.manifest_path, pkg: pkg.name, dep: dep.name });
    }
  }
  return problems;
}

function lineOf(manifestText, dep) {
  const escaped = dep.replace(/[-]/g, '\\-');
  const pattern = new RegExp(`^\\s*(${escaped}\\s*=|\\[.*dependencies\\.${escaped}\\]|.*package\\s*=\\s*"${escaped}")`);
  const index = manifestText.split('\n').findIndex((line) => pattern.test(line));
  return index + 1 || 1;
}

export function check(root) {
  const run = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (run.status !== 0) {
    return [{ file: 'Cargo.toml', line: 1, message: `cargo metadata failed: ${(run.stderr || String(run.error)).trim()}` }];
  }
  return unlistedDependencies(JSON.parse(run.stdout)).map(({ manifest, pkg, dep }) => ({
    file: relative(root, manifest).replaceAll('\\', '/'),
    line: lineOf(readFileSync(manifest, 'utf8'), dep),
    message: `${pkg} depends on ${dep}, which is not on its allowlist in scripts/check-dependencies.mjs`,
  }));
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
      `\nDependency check failed: ${errors.length} problem(s). Check that each new crate neither captures keys nor opens connections, then list it.`,
    );
    process.exit(1);
  }
  console.log('Dependency check passed.');
}
