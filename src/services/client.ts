import { invoke } from '@tauri-apps/api/core';
import type {
  Appearance,
  Choice,
  ClientError,
  ConnectRequest,
  HistoryEntry,
  NasDevice,
  NewSync,
  Preferences,
  Probe,
  Snapshot,
  SyncChange,
} from '@/types';

const CODES = new Set<ClientError>([
  'invalid_address',
  'network_unavailable',
  'nas_unreachable',
  'nas_not_configured',
  'not_a_nas',
  'trust_changed',
  'invalid_credentials',
  'too_many_attempts',
  'security_required',
  'not_paired',
  'app_missing',
  'engine_unavailable',
  'storage_unavailable',
  'secret_unavailable',
  'invalid_sync',
  'folder_taken',
  'not_connected',
  'busy',
]);

/** The error code of a failed command. An unknown answer of the native side counts as unreachable. */
export function errorCode(error: unknown): ClientError {
  const code = error instanceof Error ? error.message : error;
  return typeof code === 'string' && CODES.has(code as ClientError)
    ? (code as ClientError)
    : 'nas_unreachable';
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args);
}

export const getState = () => command<Snapshot>('get_state');

export const getHistory = (limit: number) => command<HistoryEntry[]>('get_history', { limit });

export const defaultName = () => command<string>('default_name');

export const discoverNas = (target = '') =>
  command<NasDevice[]>('discover_nas', { target: target.trim() || null });

export async function probeNas(host: string, port: number): Promise<Probe> {
  const address = host.trim();
  try {
    return await command<Probe>('probe_nas', { host: address, port });
  } catch (reason) {
    const code = errorCode(reason);
    if (code === 'nas_unreachable' || code === 'not_a_nas') {
      const devices = await discoverNas(address).catch(() => []);
      if (devices.some((device) => device.address === address && !device.configured)) {
        throw new Error('nas_not_configured', { cause: reason });
      }
    }
    throw reason;
  }
}

export const connectNas = (link: ConnectRequest) =>
  command<void>('connect_nas', { ...link, host: link.host.trim() });

export const moveNas = (host: string, port: number) =>
  command<void>('move_nas', { host: host.trim(), port });

export const disconnectNas = () => command<void>('disconnect_nas');

export const nasFolders = (path: string) => command<Choice[]>('nas_folders', { path });

export const pickFolder = () => command<string | null>('pick_folder');

export const createSync = (sync: NewSync) => command<void>('create_sync', { sync });

export const updateSync = (id: string, change: SyncChange) =>
  command<void>('update_sync', { id, change });

export const removeSync = (id: string) => command<void>('remove_sync', { id });

export const setPaused = (paused: boolean) => command<void>('set_paused', { paused });

export const savePreferences = (preferences: Preferences) =>
  command<void>('save_preferences', { preferences });

export const setAppearance = (appearance: Appearance) =>
  command<void>('set_appearance', { appearance });

export const openMain = () => command<void>('open_main');

export const hideMini = () => command<void>('hide_mini');

export const quitApp = () => command<void>('quit_app');

export function formatFingerprint(fingerprint: string) {
  return (fingerprint.toUpperCase().match(/.{1,2}/g) ?? []).join(':');
}
