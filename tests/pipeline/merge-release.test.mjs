import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

function fixture(t, version = '0.9.0') {
  const root = mkdtempSync(join(tmpdir(), 'nas-sync-client-release-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const project = join(root, 'project');
  const bin = join(root, 'bin');
  const report = join(root, 'push.json');
  mkdirSync(join(project, 'scripts/pipeline'), { recursive: true });
  mkdirSync(join(project, 'backend'));
  mkdirSync(bin);

  for (const file of [
    'merge_release.mjs',
    'prepare_release.mjs',
    'verify_release.mjs',
    'extract-changelog.cjs',
  ]) {
    copyFileSync(
      fileURLToPath(new URL(`../../scripts/pipeline/${file}`, import.meta.url)),
      join(project, 'scripts/pipeline', file),
    );
  }

  writeFileSync(join(project, 'package.json'), JSON.stringify({ type: 'module', version }));
  writeFileSync(
    join(project, 'package-lock.json'),
    JSON.stringify({ version, packages: { '': { version } } }),
  );
  writeFileSync(join(project, 'backend/Cargo.toml'), `[package]\nversion = "${version}"\n`);
  writeFileSync(join(project, 'backend/tauri.conf.json'), `{ "version": "${version}" }\n`);
  writeFileSync(join(project, 'sonar-project.properties'), `sonar.projectVersion=${version}\n`);
  writeFileSync(join(project, 'CHANGELOG.txt'), '# Changelog\n');

  const git = (...args) => execFileSync('git', args, { cwd: project, encoding: 'utf8' }).trim();
  git('init', '--initial-branch=main');
  git('config', 'user.name', 'Release Test');
  git('config', 'user.email', 'release-test@example.invalid');
  git('config', 'commit.gpgsign', 'false');
  git('config', 'tag.gpgsign', 'false');
  git('add', '.');
  git('commit', '-m', 'feat: introduce NAS discovery');

  writeFileSync(
    join(bin, 'npm'),
    `#!${process.execPath}
import { readFileSync, writeFileSync } from 'node:fs';
if (process.argv[2] === 'version') {
  const version = process.argv[3];
  for (const file of ['package.json', 'package-lock.json']) {
    const data = JSON.parse(readFileSync(file, 'utf8'));
    data.version = version;
    if (data.packages) data.packages[''].version = version;
    writeFileSync(file, JSON.stringify(data));
  }
}
`,
    { mode: 0o755 },
  );
  writeFileSync(join(bin, 'cargo'), `#!${process.execPath}\nprocess.exit(0);\n`, { mode: 0o755 });
  writeFileSync(
    join(bin, 'git'),
    `#!${process.execPath}
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
const args = process.argv.slice(2);
if (args[0] === 'push') {
  writeFileSync(process.env.RELEASE_TEST_REPORT, JSON.stringify(args));
} else {
  execFileSync('git', args, {
    env: { ...process.env, PATH: process.env.RELEASE_TEST_ORIGINAL_PATH },
    stdio: 'inherit',
  });
}
`,
    { mode: 0o755 },
  );

  const run = (...args) =>
    spawnSync(process.execPath, ['scripts/pipeline/merge_release.mjs', ...args], {
      cwd: project,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${bin}${delimiter}${process.env.PATH}`,
        RELEASE_TEST_ORIGINAL_PATH: process.env.PATH,
        RELEASE_TEST_REPORT: report,
      },
    });

  return { project, report, git, run };
}

const options = { skip: process.platform === 'win32' };

test(
  'previews the current version without tags and leaves the repository unchanged',
  options,
  (t) => {
    const { git, run } = fixture(t);
    for (const type of ['patch', 'minor', 'major']) {
      const result = run('--dry-run', `--type=${type}`);
      assert.equal(result.status, 0, result.stderr);
      assert.match(result.stdout, /Releasing v0\.9\.0 .*first release, no previous tag/);
      assert.equal(result.stderr, '');
      assert.equal(git('tag', '--list'), '');
      assert.equal(git('status', '--porcelain'), '');
    }
  },
);

test(
  'creates the first tag at the current version and includes the full commit history',
  options,
  (t) => {
    const { project, report, git, run } = fixture(t);
    const result = run();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(git('tag', '--list'), 'v0.9.0');
    assert.equal(git('rev-parse', 'v0.9.0'), git('rev-parse', 'HEAD'));
    assert.equal(git('status', '--porcelain'), '');
    const changelog = readFileSync(join(project, 'CHANGELOG.txt'), 'utf8');
    assert.match(changelog, /## \[0\.9\.0\]/);
    assert.match(changelog, /- Introduce NAS discovery\./);
    assert.deepEqual(JSON.parse(readFileSync(report, 'utf8')), [
      'push',
      'origin',
      'main',
      'v0.9.0',
    ]);
  },
);

test('keeps version bump behavior when a release tag exists', options, (t) => {
  const { git, run } = fixture(t);
  git('tag', 'v0.9.0');
  for (const [type, version] of [
    ['patch', '0.9.1'],
    ['minor', '0.10.0'],
    ['major', '1.0.0'],
  ]) {
    const result = run('--dry-run', `--type=${type}`);
    assert.equal(result.status, 0, result.stderr);
    assert.ok(result.stdout.includes(`Releasing v${version}`));
    assert.match(result.stdout, /last tag v0\.9\.0/);
  }
});

test('keeps a project version already ahead of the latest tag', options, (t) => {
  const { git, run } = fixture(t, '0.10.0');
  git('tag', 'v0.9.0');
  const result = run('--dry-run');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Releasing v0\.10\.0/);
});

test('rejects invalid release types even before the first release', options, (t) => {
  const { git, run } = fixture(t);
  const result = run('--dry-run', '--type=unknown');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Unknown release type/);
  assert.equal(git('tag', '--list'), '');
});
