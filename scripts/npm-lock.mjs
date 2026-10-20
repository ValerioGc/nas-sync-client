// Keeps package-lock.json, the lockfile the pipelines install from with `npm ci`, in step with
// package.json while the project is installed locally with pnpm.
//
//   node scripts/npm-lock.mjs --sync     # rewrites the lockfile, installs nothing
//   node scripts/npm-lock.mjs --check    # fails when package.json asks for what the lock lacks
//
// pnpm runs the --sync as `postinstall` after an install that changes something.

import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const FIELDS = ['dependencies', 'devDependencies', 'optionalDependencies'];

function fail(message) {
  console.error(`package-lock.json: ${message}`);
  process.exit(1);
}

function sync() {
  const agent = process.env.npm_config_user_agent ?? '';
  if (!agent.startsWith('pnpm/') || process.env.CI) {
    return;
  }

  const directory = mkdtempSync(join(tmpdir(), 'nas-sync-client-npm-lock-'));
  try {
    for (const file of ['package.json', 'package-lock.json', '.npmrc']) {
      if (existsSync(file)) {
        copyFileSync(file, join(directory, file));
      }
    }

    execFileSync(
      'npm',
      ['install', '--package-lock-only', '--ignore-scripts', '--no-audit', '--no-fund'],
      { cwd: directory, stdio: 'inherit' },
    );

    copyFileSync(join(directory, 'package-lock.json'), 'package-lock.json');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function check() {
  const manifest = JSON.parse(readFileSync('package.json', 'utf8'));
  let lock;
  try {
    lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
  } catch {
    fail('\n❌ Missing or unreadable: run pnpm run lock:npm');
  }

  const root = lock.packages?.[''] ?? {};
  const differences = FIELDS.flatMap((field) => {
    const wanted = manifest[field] ?? {};
    const locked = root[field] ?? {};

    return [...new Set([...Object.keys(wanted), ...Object.keys(locked)])]
      .filter((name) => wanted[name] !== locked[name])
      .map(
        (name) => `  ${name}: package.json ${wanted[name] ?? '-'}, lockfile ${locked[name] ?? '-'}`,
      );
  });

  if (differences.length > 0) {
    fail(
      `\n❌ Not in step with package.json, npm ci will refuse it:\n${differences.join('\n')}\n` +
        'run pnpm run lock:npm',
    );
  }
}

if (process.argv[2] === '--sync') {
  sync();
} else if (process.argv[2] === '--check') {
  check();
} else {
  fail('usage: node scripts/npm-lock.mjs --sync | --check');
}
