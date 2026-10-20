// Prepare the release by auditing dependencies, formatting code, running checks and tests, and building the desktop bundles.

import { execSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const CARGO = '--manifest-path backend/Cargo.toml';

const steps = [
  ['Auditing npm dependencies', 'npm audit --audit-level=high'],
  ['Formatting code', 'npm run format'],
  ['Checking formatting', 'npm run format:check'],
  ['Checking Rust formatting', `cargo fmt ${CARGO} --all --check`],
  ['Linting', 'npm run lint'],
  ['Running clippy', `cargo clippy ${CARGO} --all-targets -- -D warnings`],
  ['Running unit tests', 'npm run test:unit'],
  ['Running component tests', 'npm run test:component'],
  ['Running end-to-end tests', 'npm run test:e2e'],
  ['Running Rust tests', `cargo test ${CARGO}`],
  ['Building the interface', 'npm run build'],
  ['Verifying the frontend bundle', 'npm run test:build:check'],
  ['Building desktop bundles', 'npm run tauri:build'],
];

export function prepareRelease() {
  steps.forEach(([label, command], index) => {
    console.log(`\n▶ ${index + 1}/${steps.length} | ${label}`);
    execSync(command, { stdio: 'inherit' });
  });

  console.log('\n✅ Release preparation complete.');
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    prepareRelease();
  } catch {
    console.error('\n❌ Release preparation failed.');
    process.exit(1);
  }
}
