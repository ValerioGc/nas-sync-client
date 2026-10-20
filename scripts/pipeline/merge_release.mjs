// Merge the current release branch into the main branch, bump the version, update the changelog, and push the release tag.
//
//   npm run release:merge -- [--type=patch|minor|major] [--branch=main] [--dry-run]

import { execFileSync, execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { addChangelogEntry } from './extract-changelog.cjs';
import { prepareRelease } from './prepare_release.mjs';

const versionFiles = [
  ['backend/Cargo.toml', /^version = ".*"$/m, (version) => `version = "${version}"`],
  ['backend/tauri.conf.json', /"version": ".*"/, (version) => `"version": "${version}"`],
  [
    'sonar-project.properties',
    /^sonar\.projectVersion=.*$/m,
    (version) => `sonar.projectVersion=${version}`,
  ],
];

const option = (name) =>
  process.argv.find((argument) => argument.startsWith(`--${name}=`))?.split('=')[1];

const type = option('type') ?? 'patch';
const main = option('branch') ?? 'main';
const dryRun = process.argv.includes('--dry-run');

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
const parse = (version) => version.replace(/^v/, '').split('-')[0].split('.').map(Number);

function latestReleaseTag() {
  if (!git('tag', '--list', 'v*', '--merged', 'HEAD')) {
    return null;
  }

  return git('describe', '--tags', '--abbrev=0', '--match', 'v*');
}

/**
 * Determine the next version based on the current version, the latest tag, and the type of version bump.
 * @param {string} current - The current version.
 * @param {string | null} latest - The latest version tag, or null for the first release.
 * @param {string} bump - The type of version bump (patch, minor, major).
 * @returns {string} - The next version.
 */
function nextVersion(current, latest, bump) {
  if (!['patch', 'minor', 'major'].includes(bump)) {
    throw new Error(`Unknown release type "${bump}": use patch, minor or major.`);
  }

  if (latest === null) {
    return current;
  }

  const [major, minor, patch] = parse(latest);
  const bumps = {
    major: [major + 1, 0, 0],
    minor: [major, minor + 1, 0],
    patch: [major, minor, patch + 1],
  };

  const [ahead, behind] = [parse(current), parse(latest)];
  const first = ahead.findIndex((part, index) => part !== behind[index]);

  return first !== -1 && ahead[first] > behind[first] ? current : bumps[bump].join('.');
}

/**
 * Write the new version to the relevant files and update the changelog.
 * @param {string} version - The new version to write.
 * @param {string | null} latest - The latest version tag, or null for the first release.
 */
function writeVersion(version, latest) {
  execSync(`npm version ${version} --no-git-tag-version --allow-same-version`, { stdio: 'ignore' });
  for (const [file, pattern, line] of versionFiles) {
    writeFileSync(file, readFileSync(file, 'utf8').replace(pattern, line(version)));
  }

  const subjects = git(
    'log',
    latest === null ? 'HEAD' : `${latest}..HEAD`,
    '--no-merges',
    '--format=%s',
  ).split('\n');
  writeFileSync(
    'CHANGELOG.txt',
    addChangelogEntry(readFileSync('CHANGELOG.txt', 'utf8'), version, subjects),
  );

  execFileSync(process.execPath, ['scripts/pipeline/verify_release.mjs', `v${version}`], {
    stdio: 'inherit',
  });
}

/**
 * Merge the release branch into the main branch, update the version, and push the changes.
 * @returns {void}
 */
function mergeRelease() {
  if (git('status', '--porcelain')) {
    throw new Error('The working tree is not clean: commit or stash the changes first.');
  }

  const branch = git('rev-parse', '--abbrev-ref', 'HEAD');
  const latest = latestReleaseTag();
  const { version: current } = JSON.parse(readFileSync('package.json', 'utf8'));
  const version = nextVersion(current, latest, type);

  const baseline = latest === null ? 'first release, no previous tag' : `last tag ${latest}`;
  console.log(`\n🚀 Releasing v${version} from ${branch} into ${main} (${baseline}).`);
  if (dryRun) {
    return;
  }

  writeVersion(version, latest);
  prepareRelease();

  git('add', '--all');
  if (git('status', '--porcelain')) {
    git('commit', '-m', `chore: release v${version}`);
  }

  if (branch !== main) {
    git('checkout', main);
    git('pull', '--ff-only', 'origin', main);
    git('merge', '--no-ff', '-m', `chore: merge release v${version}`, branch);
  }

  git('tag', `v${version}`);
  git('push', 'origin', main, `v${version}`);

  console.log(`\n✅ v${version} pushed: the build pipeline takes it from here.`);
}

try {
  mergeRelease();
} catch (error) {
  console.error(`\n❌ ${error instanceof Error ? error.message : error}`);
  process.exit(1);
}
