import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { nas } from '@tests/fixtures';
import {
  connectNas,
  createSync,
  defaultName,
  disconnectNas,
  discoverNas,
  errorCode,
  formatFingerprint,
  getHistory,
  getState,
  hideMini,
  moveNas,
  nasFolders,
  openMain,
  pickFolder,
  probeNas,
  quitApp,
  removeSync,
  savePreferences,
  setAppearance,
  setPaused,
  updateSync,
} from '@/services/client';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

it('sends every command with its typed arguments', async () => {
  const calls: [() => Promise<unknown>, string, unknown][] = [
    [getState, 'get_state', undefined],
    [() => getHistory(8), 'get_history', { limit: 8 }],
    [defaultName, 'default_name', undefined],
    [() => discoverNas(' 192.168.1.15 '), 'discover_nas', { target: '192.168.1.15' }],
    [() => discoverNas(), 'discover_nas', { target: null }],
    [() => probeNas(' nas.local ', 443), 'probe_nas', { host: 'nas.local', port: 443 }],
    [() => moveNas('nas', 8443), 'move_nas', { host: 'nas', port: 8443 }],
    [disconnectNas, 'disconnect_nas', undefined],
    [() => nasFolders('v-1'), 'nas_folders', { path: 'v-1' }],
    [pickFolder, 'pick_folder', undefined],
    [() => removeSync('f-1'), 'remove_sync', { id: 'f-1' }],
    [() => setPaused(true), 'set_paused', { paused: true }],
    [
      () => updateSync('f-1', { paused: false }),
      'update_sync',
      { id: 'f-1', change: { paused: false } },
    ],
    [openMain, 'open_main', undefined],
    [hideMini, 'hide_mini', undefined],
    [quitApp, 'quit_app', undefined],
  ];
  for (const [call, name, args] of calls) {
    await call();
    expect(invoke).toHaveBeenLastCalledWith(name, args);
  }
  await connectNas({
    host: ' nas ',
    port: 443,
    fingerprint: 'ff',
    username: 'admin',
    password: 'secret',
    name: 'PC',
  });
  expect(invoke).toHaveBeenLastCalledWith('connect_nas', expect.objectContaining({ host: 'nas' }));
  const sync = { name: 'A', nasPath: 'v-1/A', pcPath: '/a', mode: 'two-way' as const };
  await createSync(sync);
  expect(invoke).toHaveBeenLastCalledWith('create_sync', { sync });
  const preferences = { notifications: true, autostart: true, maxSendKbps: 1, maxRecvKbps: 2 };
  await savePreferences(preferences);
  expect(invoke).toHaveBeenLastCalledWith('save_preferences', { preferences });
  await setAppearance({ locale: 'en', theme: 'light' });
  expect(invoke).toHaveBeenLastCalledWith('set_appearance', {
    appearance: { locale: 'en', theme: 'light' },
  });
});

it('keeps the known error codes and formats the fingerprint like the NAS', () => {
  expect(errorCode('folder_taken')).toBe('folder_taken');
  expect(errorCode(new Error('details of the machine'))).toBe('nas_unreachable');
  expect(formatFingerprint('ab01ff')).toBe('AB:01:FF');
  expect(formatFingerprint('')).toBe('');
});

it.each(['nas_unreachable', 'not_a_nas'])(
  'recognizes a NAS awaiting setup when HTTPS verification returns %s',
  async (reason) => {
    vi.mocked(invoke).mockRejectedValueOnce(reason);
    vi.mocked(invoke).mockResolvedValueOnce([nas({ configured: false })]);
    await expect(probeNas(' 192.168.1.15 ', 443)).rejects.toThrow('nas_not_configured');
    expect(invoke).toHaveBeenLastCalledWith('discover_nas', { target: '192.168.1.15' });
    expect(errorCode('nas_not_configured')).toBe('nas_not_configured');
    expect(errorCode(new Error('nas_not_configured'))).toBe('nas_not_configured');
  },
);

it.each([
  { devices: [] },
  { devices: [nas()] },
  { devices: [nas({ address: '192.168.1.99', configured: false })] },
])(
  'preserves the verification failure without a matching NAS awaiting setup: %j',
  async ({ devices }) => {
    vi.mocked(invoke).mockRejectedValueOnce('nas_unreachable');
    vi.mocked(invoke).mockResolvedValueOnce(devices);
    await expect(probeNas('192.168.1.15', 443)).rejects.toBe('nas_unreachable');
  },
);

it('preserves the verification failure when discovery is unavailable', async () => {
  vi.mocked(invoke).mockRejectedValueOnce('not_a_nas');
  vi.mocked(invoke).mockRejectedValueOnce('network_unavailable');
  await expect(probeNas('192.168.1.15', 443)).rejects.toBe('not_a_nas');
});

it('does not run discovery for invalid input', async () => {
  vi.mocked(invoke).mockRejectedValueOnce('invalid_address');
  await expect(probeNas('invalid host', 443)).rejects.toBe('invalid_address');
  expect(invoke).toHaveBeenCalledTimes(1);
});
