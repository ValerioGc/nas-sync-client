import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const { modules } = JSON.parse(readFileSync('build-reports/tauri.json', 'utf8'));
const javascript = readdirSync('dist/tauri/assets')
  .filter((name) => name.endsWith('.js'))
  .map((name) => readFileSync(join('dist/tauri/assets', name), 'utf8'))
  .join('\n');

test('desktop bundle includes the native transport, the window controls and the tray summary', () => {
  for (const id of [
    'src/main.ts',
    'src/App.vue',
    'src/services/client.ts',
    'src/components/TitleBar.vue',
    'src/components/MiniView.vue',
  ]) {
    assert.ok(modules.includes(id), `Missing desktop module: ${id}`);
  }
  assert.ok(modules.some((id) => id.includes('@tauri-apps')));
  assert.ok(javascript.includes('connect_nas'));
  assert.ok(javascript.includes('set_paused'));
});

test('production output excludes test helpers and selectors', () => {
  assert.ok(!modules.some((id) => id.startsWith('tests/')));
  assert.ok(!javascript.includes('data-testid'));
  assert.ok(!javascript.includes('__testTauri'));
});
