import { test as base } from 'playwright-bdd';
import type { HistoryEntry, Snapshot, SyncView } from '../../../src/types';

interface ClientFixture {
  snapshot: Snapshot;
  history: HistoryEntry[];
  credentials: string;
}

declare global {
  interface Window {
    __clientFixture: ClientFixture;

    __emitClientState: () => void;
  }
}

export const test = base.extend<{ nativeClient: void }>({
  nativeClient: [
    async ({ page }, use) => {
      await page.addInitScript(() => {
        const fixture: ClientFixture = {
          snapshot: {
            status: 'unconfigured',
            problem: null,
            link: null,
            paused: false,
            syncs: [],
            preferences: { notifications: true, autostart: false, maxSendKbps: 0, maxRecvKbps: 0 },
            appearance: { locale: 'en', theme: 'dark' },
          },
          history: [],
          credentials: 'secret',
        };
        window.__clientFixture = fixture;
        const state = () => fixture.snapshot;
        const status = () => {
          const current = state();
          if (!current.link) {
            current.status = 'unconfigured';
          } else if (current.paused) {
            current.status = 'paused';
          } else if (current.problem) {
            current.status = 'disconnected';
          } else {
            current.status = 'synced';
          }
        };
        const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
          get_state: () => {
            status();
            return structuredClone(state());
          },
          get_history: () => fixture.history,
          default_name: () => 'studio-pc',
          discover_nas: () => [
            {
              id: 'a'.repeat(64),
              name: 'nas-studio',
              address: '192.168.1.15',
              port: 443,
              secure: true,
              configured: true,
            },
          ],
          probe_nas: (args) => {
            if (args.host !== '192.168.1.15') {
              throw 'nas_unreachable';
            }
            return { fingerprint: 'ab'.repeat(32) };
          },
          connect_nas: (args) => {
            if (args.password !== fixture.credentials) {
              throw 'invalid_credentials';
            }
            state().link = {
              name: String(args.name),
              host: String(args.host),
              port: Number(args.port),
              fingerprint: String(args.fingerprint),
            };
          },
          move_nas: (args) => {
            const link = state().link;
            if (link) {
              link.host = String(args.host);
            }
          },
          disconnect_nas: () => {
            state().link = null;
            state().syncs = [];
          },
          nas_folders: (args) =>
            args.path ? [{ name: 'Photos', path: 'v-1/Photos' }] : [{ name: 'Data', path: 'v-1' }],
          pick_folder: () => '/home/me/Photos',
          create_sync: (args) => {
            const sync = args.sync as Omit<SyncView, 'id'>;
            state().syncs.push({
              ...sync,
              id: `f-${state().syncs.length + 1}`,
              nasLabel: sync.nasPath.replace('v-1', 'Data'),
              state: 'synced',
              progress: 100,
              lastSync: null,
            } as SyncView);
          },
          update_sync: (args) => {
            const change = args.change as { paused?: boolean };
            const sync = state().syncs.find((item) => item.id === args.id);
            if (sync && change.paused !== undefined) {
              sync.state = change.paused ? 'paused' : 'synced';
            }
          },
          remove_sync: (args) => {
            state().syncs = state().syncs.filter((item) => item.id !== args.id);
          },
          set_paused: (args) => {
            state().paused = Boolean(args.paused);
          },
        };

        const callbacks = new Map<number, (event: unknown) => void>();
        const listening: number[] = [];
        let nextCallback = 1;
        handlers['plugin:event|listen'] = (args) => {
          listening.push(args.handler as number);
          return args.handler;
        };
        handlers['plugin:event|unlisten'] = () => undefined;
        window.__emitClientState = () => {
          for (const id of listening) {
            callbacks.get(id)?.({ event: 'client-state', id, payload: null });
          }
        };
        Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
          value: { unregisterListener: () => undefined },
        });
        Object.defineProperty(window, '__TAURI_INTERNALS__', {
          value: {
            invoke: async (command: string, args: Record<string, unknown> = {}) => {
              return handlers[command]?.(args);
            },
            transformCallback: (callback: (event: unknown) => void) => {
              const id = nextCallback++;
              callbacks.set(id, callback);
              return id;
            },
            unregisterCallback: (id: number) => callbacks.delete(id),
          },
        });
      });
      await use();
    },
    { auto: true },
  ],
});
