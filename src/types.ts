import type en from '@/i18n/lang/en';

export type MessageKey = keyof typeof en;

export type ClientError =
  | 'invalid_address'
  | 'network_unavailable'
  | 'nas_unreachable'
  | 'nas_not_configured'
  | 'not_a_nas'
  | 'trust_changed'
  | 'invalid_credentials'
  | 'too_many_attempts'
  | 'security_required'
  | 'not_paired'
  | 'app_missing'
  | 'engine_unavailable'
  | 'storage_unavailable'
  | 'secret_unavailable'
  | 'invalid_sync'
  | 'folder_taken'
  | 'not_connected'
  | 'busy';

export type Status = 'unconfigured' | 'paused' | 'disconnected' | 'syncing' | 'synced' | 'error';

export type SyncMode = 'two-way' | 'pc-to-nas' | 'nas-to-pc';

export type SyncState =
  'synced' | 'syncing' | 'paused' | 'offline' | 'error' | 'revoked' | 'unchosen';

export type Locale = 'it' | 'en';

export type Theme = 'dark' | 'light';
export type MainTab = 'syncs' | 'history';
export type WizardStep = 'address' | 'trust' | 'account';

export interface LinkView {
  name: string;
  host: string;
  port: number;
  fingerprint: string;
}

export interface SyncView {
  id: string;
  name: string;
  nasPath: string;
  nasLabel: string;
  pcPath: string;
  mode: SyncMode;
  state: SyncState;
  progress: number | null;
  lastSync: string | null;
}

export interface Preferences {
  notifications: boolean;
  autostart: boolean;
  maxSendKbps: number;
  maxRecvKbps: number;
}

export interface Appearance {
  locale: Locale;
  theme: Theme;
}

export interface Snapshot {
  status: Status;
  problem: ClientError | null;
  link: LinkView | null;
  paused: boolean;
  syncs: SyncView[];
  preferences: Preferences;
  appearance: Appearance;
}

export interface HistoryEntry {
  time: string;
  folder: string;
  path: string;
  action: 'added' | 'modified' | 'deleted' | 'failed';
  direction: 'upload' | 'download';
  conflict: boolean;
}

export interface NasDevice {
  id: string;
  name: string;
  address: string;
  port: number;
  secure: boolean;
  configured: boolean;
}

export interface Choice {
  name: string;
  path: string;
}

export interface NewSync {
  name: string;
  nasPath: string;
  pcPath: string;
  mode: SyncMode;
}

export interface SyncChange {
  name?: string;
  pcPath?: string;
  mode?: SyncMode;
  paused?: boolean;
}

export interface Probe {
  fingerprint: string;
}

export interface ConnectRequest {
  host: string;
  port: number;
  fingerprint: string;
  username: string;
  password: string;
  name: string;
}

export interface DropdownItem {
  id: string;
  label: string;
  icon: string;
  disabled?: boolean;
}

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}
