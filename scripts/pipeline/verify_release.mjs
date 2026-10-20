// Verify that the release metadata matches the expected versions and changelog.

import { readFileSync } from 'node:fs';
import { extractChangelog } from './extract-changelog.cjs';

const tag = process.argv[2] ?? `v${JSON.parse(readFileSync('package.json', 'utf8')).version}`;
const semver = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function cargoVersion(path) {
  const match = readFileSync(path, 'utf8').match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`\n❌ ${path} has no package version`);
  }

  return match[1];
}

try {
  if (!semver.test(tag)) {
    throw new Error(`\n❌ Invalid release tag: ${tag}`);
  }

  const expected = tag.slice(1);
  const versions = {
    'package.json': JSON.parse(readFileSync('package.json', 'utf8')).version,
    'package-lock.json': JSON.parse(readFileSync('package-lock.json', 'utf8')).version,
    'package-lock root': JSON.parse(readFileSync('package-lock.json', 'utf8')).packages[''].version,
    'backend/tauri.conf.json': JSON.parse(readFileSync('backend/tauri.conf.json', 'utf8')).version,
    'backend/Cargo.toml': cargoVersion('backend/Cargo.toml'),
  };

  for (const [file, version] of Object.entries(versions)) {
    if (version !== expected) {
      throw new Error(`\n❌ ${file} has version ${version}; expected ${expected}`);
    }
  }

  extractChangelog(expected);
  console.log(`\n✅ Release metadata matches ${tag}.`);
} catch (error) {
  console.error(`\n❌ Release metadata check failed: ${error.message}`);
  process.exitCode = 1;
}
