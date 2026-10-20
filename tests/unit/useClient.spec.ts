import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { REFRESH_MS, STATE_EVENT, useClient } from '@/composables/useClient';
import { answer, entry, snapshot } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
const unlisten = vi.fn();

function host() {
  let client!: ReturnType<typeof useClient>;
  const wrapper = mount(
    defineComponent({
      setup() {
        client = useClient(5);
        return () => h('div');
      },
    }),
  );
  return { wrapper, client: () => client };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.mocked(invoke).mockReset();
  vi.mocked(listen).mockReset().mockResolvedValue(unlisten);
  answer({
    get_state: snapshot({ appearance: { locale: 'en', theme: 'light' } }),
    get_history: [entry()],
  });
});

afterEach(() => vi.useRealTimers());

it('reads the state at once and then while the window is visible', async () => {
  const { wrapper, client } = host();
  await flushPromises();
  expect(client().snapshot.value?.status).toBe('synced');
  expect(client().history.value).toHaveLength(1);
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(document.documentElement.lang).toBe('en');
  expect(invoke).toHaveBeenCalledWith('get_history', { limit: 5 });

  vi.mocked(invoke).mockClear();
  Object.defineProperty(document, 'hidden', { configurable: true, value: true });
  vi.advanceTimersByTime(REFRESH_MS);
  expect(invoke).not.toHaveBeenCalled();
  Object.defineProperty(document, 'hidden', { configurable: true, value: false });
  vi.advanceTimersByTime(REFRESH_MS);
  expect(invoke).toHaveBeenCalledWith('get_state', undefined);
  wrapper.unmount();
});

it('reads the state again at each event of the client, and stops listening when it goes', async () => {
  const { wrapper } = host();
  await flushPromises();
  expect(listen).toHaveBeenCalledWith(STATE_EVENT, expect.any(Function));
  const handler = vi.mocked(listen).mock.calls[0]![1] as () => void;
  vi.mocked(invoke).mockClear();
  handler();
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('get_state', undefined);
  wrapper.unmount();
  expect(unlisten).toHaveBeenCalledOnce();
});

it('reads no history before a NAS is linked, and works without events', async () => {
  vi.mocked(listen).mockRejectedValue(new Error('not in the application'));
  answer({ get_state: snapshot({ status: 'unconfigured' }), get_history: [entry()] });
  const { wrapper, client } = host();
  await flushPromises();
  expect(client().snapshot.value?.status).toBe('unconfigured');
  expect(client().history.value).toEqual([]);
  expect(invoke).not.toHaveBeenCalledWith('get_history', expect.anything());
  wrapper.unmount();
});

it('runs one change at a time and keeps the code of a failure', async () => {
  const { wrapper, client } = host();
  await flushPromises();
  let finish!: () => void;
  const first = client().run(() => new Promise<void>((resolve) => (finish = resolve)));
  await nextTick();
  expect(client().busy.value).toBe(true);
  expect(await client().run(async () => undefined)).toBe(false);
  finish();
  expect(await first).toBe(true);
  expect(await client().run(() => Promise.reject('busy'))).toBe(false);
  expect(client().error.value).toBe('busy');

  vi.mocked(invoke).mockRejectedValue('engine_unavailable');
  await client().refresh();
  expect(client().error.value).toBe('engine_unavailable');
  wrapper.unmount();
});
