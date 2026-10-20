import { vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryEntry, NasDevice, Snapshot, SyncView } from '@/types';

export function nas(overrides: Partial<NasDevice> = {}): NasDevice {
  return {
    id: 'a'.repeat(64),
    name: 'nas-studio',
    address: '192.168.1.15',
    port: 443,
    secure: true,
    configured: true,
    ...overrides,
  };
}

export function sync(overrides: Partial<SyncView> = {}): SyncView {
  return {
    id: 'f-1',
    name: 'Photos',
    nasPath: 'v-1/Photos',
    nasLabel: 'Data/Photos',
    pcPath: '/home/me/Photos',
    mode: 'two-way',
    state: 'synced',
    progress: 100,
    lastSync: '2026-10-16T10:00:00Z',
    ...overrides,
  };
}

export function entry(overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    time: '2026-10-16T10:00:00Z',
    folder: 'f-1',
    path: 'docs/report.pdf',
    action: 'modified',
    direction: 'upload',
    conflict: false,
    ...overrides,
  };
}

export function snapshot(overrides: Partial<Snapshot> = {}): Snapshot {
  return {
    status: 'synced',
    problem: null,
    link: { name: 'Studio PC', host: '192.168.1.15', port: 443, fingerprint: 'ab'.repeat(32) },
    paused: false,
    syncs: [sync()],
    preferences: { notifications: true, autostart: false, maxSendKbps: 0, maxRecvKbps: 0 },
    appearance: { locale: 'en', theme: 'dark' },
    ...overrides,
  };
}

export function answer(answers: Record<string, unknown>) {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    const value = answers[command];
    if (typeof value === 'function') {
      return value(args);
    }

    return value;
  });
}
