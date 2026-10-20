// Puts the official Syncthing binary of the platform beside the client, as the sidecar Tauri
// bundles (backend/binaries/syncthing-<target>), after checking it against the SHA-256 the
// release publishes. The version is the one Debian 13 ships on the NAS.
//
//   node scripts/build/engine.mjs          # downloads it once, then keeps it
//
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const VERSION = '1.29.5';

export const ENGINES = {
  linux: {
    target: 'x86_64-unknown-linux-gnu',
    file: `syncthing-linux-amd64-v${VERSION}.tar.gz`,
    sha256: 'b05cb12f6f58309612194cf1126b5ac090525e2c40e6bd64cc59b347dccbe441',
    binary: 'syncthing',
  },
  win32: {
    target: 'x86_64-pc-windows-msvc',
    file: `syncthing-windows-amd64-v${VERSION}.zip`,
    sha256: '5c7347c6c32b39654e9f60772135eeaa76438a9d627ffb0fb41fe15f8aecfd0d',
    binary: 'syncthing.exe',
  },
};

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** Where the sidecar of a platform goes. */
export function sidecar(engine, root = ROOT) {
  const extension = engine.binary.endsWith('.exe') ? '.exe' : '';
  return join(root, 'backend', 'binaries', `syncthing-${engine.target}${extension}`);
}

/** Refuses an archive whose SHA-256 is not the one of the release. */
export function verify(bytes, expected) {
  const actual = createHash('sha256').update(bytes).digest('hex');
  if (actual !== expected) {
    throw new Error(`checksum mismatch: expected ${expected}, got ${actual}`);
  }
}

/** Archive directory name without its extension. */
export function folderOf(engine) {
  return engine.file.replace(/\.(tar\.gz|zip)$/, '');
}

export async function fetchEngine(platform = process.platform, root = ROOT) {
  const engine = ENGINES[platform];
  if (!engine) {
    throw new Error(`no engine for ${platform}`);
  }
  const target = sidecar(engine, root);
  if (existsSync(target)) {
    return target;
  }
  const url = `https://github.com/syncthing/syncthing/releases/download/v${VERSION}/${engine.file}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`cannot download ${url}: ${response.status}`);
  }
  const bytes = Buffer.from(await response.arrayBuffer());
  verify(bytes, engine.sha256);
  const work = mkdtempSync(join(tmpdir(), 'nas-sync-engine-'));
  try {
    const archive = join(work, engine.file);
    writeFileSync(archive, bytes);
    execFileSync('tar', ['-xf', archive, '-C', work]);
    const binary = join(work, folderOf(engine), engine.binary);
    if (!existsSync(binary)) {
      throw new Error(`${engine.binary} is missing from ${engine.file}`);
    }
    mkdirSync(join(root, 'backend', 'binaries'), { recursive: true });
    copyFileSync(binary, target);
    chmodSync(target, 0o755);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
  return target;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  fetchEngine()
    .then((target) => console.log(`Syncthing ${VERSION}: ${target}`))
    .catch((error) => {
      console.error(`Syncthing engine: ${error.message}`);
      process.exit(1);
    });
}
