import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('../../scripts/npm-lock.mjs', import.meta.url));

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'npm-lock-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const project = join(root, 'project');
  const bin = join(root, 'bin');
  mkdirSync(project);
  mkdirSync(bin);
  mkdirSync(join(project, 'node_modules'));
  symlinkSync('missing-pnpm-target', join(project, 'node_modules', 'example'));
  writeFileSync(join(project, 'package.json'), '{"name":"fixture","version":"1.0.0"}\n');
  writeFileSync(join(project, '.npmrc'), 'save-exact=true\n');
  const report = join(root, 'report.json');
  writeFileSync(
    join(bin, 'npm'),
    `#!${process.execPath}
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
writeFileSync(process.env.LOCK_TEST_REPORT, JSON.stringify({
  cwd: process.cwd(),
  args: process.argv.slice(2),
  hasModules: existsSync('node_modules'),
  manifest: readFileSync('package.json', 'utf8'),
  config: readFileSync('.npmrc', 'utf8'),
  oldLock: existsSync('package-lock.json') ? readFileSync('package-lock.json', 'utf8') : null,
}));
if (process.env.LOCK_TEST_FAIL) process.exit(1);
writeFileSync('package-lock.json', '{"lockfileVersion":3,"packages":{"":{}}}\\n');
`,
    { mode: 0o755 },
  );
  const run = (env = {}, mode = '--sync') =>
    spawnSync(process.execPath, [script, mode], {
      cwd: project,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${bin}${delimiter}${process.env.PATH}`,
        npm_config_user_agent: 'pnpm/11',
        CI: '',
        LOCK_TEST_REPORT: report,
        ...env,
      },
    });
  return { project, report, run };
}

const options = { skip: process.platform === 'win32' };

test('sync generates a missing lock without inspecting pnpm node_modules', options, (t) => {
  const { project, report, run } = fixture(t);
  const result = run();
  assert.equal(result.status, 0, result.stderr);
  const invocation = JSON.parse(readFileSync(report, 'utf8'));
  assert.notEqual(invocation.cwd, project);
  assert.equal(invocation.hasModules, false);
  assert.equal(invocation.config, 'save-exact=true\n');
  assert.equal(invocation.manifest, readFileSync(join(project, 'package.json'), 'utf8'));
  assert.equal(invocation.oldLock, null);
  assert.deepEqual(invocation.args, [
    'install',
    '--package-lock-only',
    '--ignore-scripts',
    '--no-audit',
    '--no-fund',
  ]);
  assert.equal(existsSync(invocation.cwd), false);
  const link = join(project, 'node_modules', 'example');
  assert.equal(lstatSync(link).isSymbolicLink(), true);
  assert.equal(readlinkSync(link), 'missing-pnpm-target');
  assert.equal(run({}, '--check').status, 0);
});

test('sync seeds npm with the existing lock and preserves it if npm fails', options, (t) => {
  const { project, report, run } = fixture(t);
  const original = '{"lockfileVersion":3,"packages":{"":{}},"marker":"pinned"}\n';
  const lock = join(project, 'package-lock.json');
  writeFileSync(lock, original);
  const failure = run({ LOCK_TEST_FAIL: '1' });
  assert.notEqual(failure.status, 0);
  let invocation = JSON.parse(readFileSync(report, 'utf8'));
  assert.equal(invocation.oldLock, original);
  assert.equal(readFileSync(lock, 'utf8'), original);
  assert.equal(existsSync(invocation.cwd), false);
  const success = run();
  assert.equal(success.status, 0, success.stderr);
  invocation = JSON.parse(readFileSync(report, 'utf8'));
  assert.equal(invocation.oldLock, original);
  assert.equal(existsSync(invocation.cwd), false);
  assert.notEqual(readFileSync(lock, 'utf8'), original);
});

test('sync skips npm installs and CI', options, (t) => {
  const { project, report, run } = fixture(t);
  for (const env of [{ npm_config_user_agent: 'npm/11' }, { CI: 'true' }]) {
    const result = run(env);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(report), false);
    assert.equal(existsSync(join(project, 'package-lock.json')), false);
  }
});
