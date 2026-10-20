import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import SearchNasModal from '@/components/SearchNasModal.vue';
import { setLocale } from '@/i18n/i18n';
import { answer, nas } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  setLocale('en');
  vi.mocked(invoke).mockReset();
});

const retry = () =>
  document.body.querySelector<HTMLButtonElement>('[data-testid="search_submit"]')?.click();

it('finds the NAS of the network and lets a configured one be chosen', async () => {
  answer({
    discover_nas: [nas(), nas({ id: 'b'.repeat(64), name: 'new-nas', configured: false })],
  });
  const wrapper = mount(SearchNasModal);
  await flushPromises();
  expect(document.body.textContent).toContain('Network discovery');
  expect(invoke).toHaveBeenCalledWith('discover_nas', { target: null });
  expect(document.body.querySelector('dialog input')).toBeNull();
  const rows = document.body.querySelectorAll('[data-testid="nas_row"]');
  expect(rows).toHaveLength(2);
  expect(rows[1]?.textContent).toContain('Initial setup required');
  expect(rows[1]?.textContent).toContain('The NAS was detected successfully');
  expect(rows[1]?.querySelector('button')?.disabled).toBe(true);
  rows[0]?.querySelector('button')?.click();
  expect(wrapper.emitted('select')).toEqual([[nas()]]);
  document.body.querySelector<HTMLButtonElement>('dialog button[aria-label="Close"]')?.click();
  expect(wrapper.emitted('close')).toHaveLength(1);
  wrapper.unmount();
});

it('says when nothing answers and allows a failed search to be retried', async () => {
  answer({ discover_nas: [] });
  const wrapper = mount(SearchNasModal);
  await flushPromises();
  expect(document.body.textContent).toContain('No NAS found');
  vi.mocked(invoke).mockRejectedValueOnce('network_unavailable');
  retry();
  await flushPromises();
  expect(document.body.querySelector('[role="alert"]')?.textContent).toContain(
    'Network unavailable',
  );
  retry();
  await flushPromises();
  expect(document.body.querySelector('[role="alert"]')).toBeNull();
  expect(document.body.textContent).toContain('No NAS found');
  wrapper.unmount();
});

it('searches once at a time', async () => {
  let finish!: (value: unknown) => void;
  vi.mocked(invoke).mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  const wrapper = mount(SearchNasModal);
  await flushPromises();
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(document.body.querySelector('output .spinner')).not.toBeNull();
  expect(document.body.querySelector('[data-testid="search_submit"]')).toBeNull();
  expect(document.body.textContent).toContain('Searching');
  finish([]);
  await flushPromises();
  wrapper.unmount();
});
