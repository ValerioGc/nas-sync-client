import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { ENGINES, VERSION, folderOf, sidecar, verify } from '../../scripts/build/engine.mjs';

test('the engine of each platform is pinned to the release and to its checksum', () => {
  for (const engine of Object.values(ENGINES)) {
    assert.match(engine.file, new RegExp(`v${VERSION.replaceAll('.', '\\.')}`));
    assert.match(engine.sha256, /^[0-9a-f]{64}$/);
  }
  assert.ok(sidecar(ENGINES.linux, '/project').endsWith('syncthing-x86_64-unknown-linux-gnu'));
  assert.ok(sidecar(ENGINES.win32, '/project').endsWith('syncthing-x86_64-pc-windows-msvc.exe'));
  assert.equal(folderOf(ENGINES.linux), `syncthing-linux-amd64-v${VERSION}`);
  assert.equal(folderOf(ENGINES.win32), `syncthing-windows-amd64-v${VERSION}`);
});

test('an archive with another checksum is refused', () => {
  const bytes = Buffer.from('engine');
  verify(bytes, createHash('sha256').update(bytes).digest('hex'));
  assert.throws(() => verify(bytes, '0'.repeat(64)), /checksum mismatch/);
});
