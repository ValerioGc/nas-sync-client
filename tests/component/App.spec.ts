import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from '@/App.vue';
import { answer, entry, snapshot, sync } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const body = () => document.body.textContent ?? '';
const byTestId = (testid: string) =>
  document.body.querySelector<HTMLElement>(`[data-testid="${testid}"]`);
const buttonNamed = (name: string) =>
  [...document.body.querySelectorAll('button')].find(
    (button) => button.textContent?.trim() === name,
  );

async function app(state = snapshot(), more: Record<string, unknown> = {}) {
  answer({
    get_state: state,
    get_history: [entry()],
    nas_folders: [],
    default_name: 'studio-pc',
    ...more,
  });
  const wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  return wrapper;
}

it('shows the wizard until a NAS is connected', async () => {
  const wrapper = await app(snapshot({ link: null, status: 'unconfigured', syncs: [] }));
  expect(byTestId('connect_wizard')).not.toBeNull();
  expect(byTestId('open_options')).toBeNull();
  wrapper.unmount();
});

it('shows the state, the synced folders and the history of a connected NAS', async () => {
  const wrapper = await app(
    snapshot({
      status: 'disconnected',
      problem: 'nas_unreachable',
      syncs: [sync(), sync({ id: 'f-2', name: 'Docs' })],
    }),
  );
  expect(byTestId('status_badge')?.textContent).toContain('Not connected');
  expect(body()).toContain('NAS unreachable. Check the network connection');
  expect(document.body.querySelectorAll('[data-testid="sync_row"]')).toHaveLength(2);
  expect(byTestId('panel_history')).toBeNull();
  expect(byTestId('tab_syncs')?.getAttribute('aria-selected')).toBe('true');

  byTestId('tab_history')?.click();
  await flushPromises();
  expect(byTestId('panel_syncs')).toBeNull();
  expect(byTestId('new_sync')).toBeNull();
  expect(document.body.querySelectorAll('[data-testid="history_entry"]')).toHaveLength(1);
  byTestId('tab_history')?.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }),
  );
  await flushPromises();
  expect(byTestId('panel_syncs')).not.toBeNull();
  expect(document.activeElement).toBe(byTestId('tab_syncs'));
  byTestId('tab_syncs')?.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }),
  );
  await flushPromises();
  expect(byTestId('tab_history')?.getAttribute('aria-selected')).toBe('true');
  wrapper.unmount();
});

it('says when there is no synced folder', async () => {
  const wrapper = await app(snapshot({ syncs: [] }));
  expect(body()).toContain('No synced folders');
  wrapper.unmount();
});

it('pauses everything and a single sync, and reports a refusal', async () => {
  const wrapper = await app(snapshot(), { set_paused: undefined, update_sync: undefined });
  byTestId('toggle_all')?.click();
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('set_paused', { paused: true });
  document.body.querySelector<HTMLButtonElement>('button[aria-label="Pause Photos"]')?.click();
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('update_sync', { id: 'f-1', change: { paused: true } });
  answer({
    get_state: snapshot({ paused: true }),
    get_history: [],
    set_paused: () => Promise.reject('busy'),
  });
  byTestId('toggle_all')?.click();
  await flushPromises();
  expect(body()).toContain('An operation is already running');
  wrapper.unmount();
});

it('creates, edits and removes a sync', async () => {
  const wrapper = await app(snapshot(), {
    pick_folder: '/home/me/Music',
    nas_folders: ({ path }: { path: string }) => (path ? [] : [{ name: 'Data', path: 'v-1' }]),
    create_sync: undefined,
    update_sync: undefined,
    remove_sync: undefined,
  });
  byTestId('new_sync')?.click();
  await flushPromises();
  byTestId('rule_browse')?.click();
  await flushPromises();
  byTestId('nas_folder')?.click();
  await flushPromises();
  document.body.querySelector('dialog form')?.dispatchEvent(new Event('submit'));
  await flushPromises();
  expect(invoke).not.toHaveBeenCalledWith('create_sync', expect.anything());

  buttonNamed('Cancel')?.click();
  await flushPromises();
  document.body.querySelector<HTMLButtonElement>('button[aria-label="Edit Photos"]')?.click();
  await flushPromises();
  document.body.querySelector('dialog form')?.dispatchEvent(new Event('submit'));
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('update_sync', {
    id: 'f-1',
    change: { name: 'Photos', pcPath: '/home/me/Photos', mode: 'two-way' },
  });
  expect(document.body.querySelector('dialog')).toBeNull();

  document.body.querySelector<HTMLButtonElement>('button[aria-label="Remove Photos"]')?.click();
  await flushPromises();
  byTestId('confirm_action')?.click();
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('remove_sync', { id: 'f-1' });
  expect(document.body.querySelector('dialog')).toBeNull();
  wrapper.unmount();
});

it('creates a sync on a folder of a disk', async () => {
  const wrapper = await app(snapshot({ syncs: [] }), {
    pick_folder: '/home/me/Music',
    nas_folders: ({ path }: { path: string }) =>
      path ? [{ name: 'Music', path: 'v-1/Music' }] : [{ name: 'Data', path: 'v-1' }],
    create_sync: undefined,
  });
  byTestId('new_sync')?.click();
  await flushPromises();
  byTestId('rule_browse')?.click();
  await flushPromises();
  byTestId('nas_folder')?.click();
  await flushPromises();
  byTestId('nas_folder')?.click();
  await flushPromises();
  document.body.querySelector('dialog form')?.dispatchEvent(new Event('submit'));
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('create_sync', {
    sync: { name: 'Music', nasPath: 'v-1/Music', pcPath: '/home/me/Music', mode: 'two-way' },
  });
  wrapper.unmount();
});

it('changes the options, the address of the NAS and disconnects it', async () => {
  const wrapper = await app(snapshot(), {
    save_preferences: undefined,
    move_nas: undefined,
    disconnect_nas: undefined,
  });
  const options = async () => {
    byTestId('open_options')?.click();
    await flushPromises();
  };
  await options();
  document.getElementById('options_form')?.dispatchEvent(new Event('submit'));
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('save_preferences', {
    preferences: { notifications: true, autostart: false, maxSendKbps: 0, maxRecvKbps: 0 },
  });
  await options();
  byTestId('options_move')?.closest('form')?.dispatchEvent(new Event('submit'));
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('move_nas', { host: '192.168.1.15', port: 443 });
  await options();
  byTestId('options_disconnect')?.click();
  await flushPromises();
  byTestId('confirm_action')?.click();
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('disconnect_nas', undefined);
  await options();
  document.body.querySelector<HTMLButtonElement>('dialog button[aria-label="Close"]')?.click();
  await flushPromises();
  expect(document.body.querySelector('dialog')).toBeNull();
  wrapper.unmount();
});

it('shows the loading state while the first state is read', async () => {
  vi.mocked(invoke).mockImplementation(() => new Promise(() => undefined));
  const wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  expect(byTestId('main_view')?.textContent).toMatch(/Caricamento|Loading/);
  wrapper.unmount();
});
