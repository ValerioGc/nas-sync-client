import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import MiniView from '@/components/MiniView.vue';
import { answer, entry, snapshot } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

async function mini(state = snapshot()) {
  answer({ get_state: state, get_history: [entry()], set_paused: undefined });
  const wrapper = mount(MiniView, { attachTo: document.body });
  await flushPromises();
  return wrapper;
}

it('shows the state of the sync and the last updated files', async () => {
  const wrapper = await mini(snapshot({ status: 'disconnected', problem: 'not_connected' }));
  expect(invoke).toHaveBeenCalledWith('get_history', { limit: 8 });
  expect(wrapper.get('[data-testid="status_badge"]').text()).toBe('Not connected');
  expect(wrapper.text()).toContain('Check the network connection');
  expect(wrapper.text()).toContain('report.pdf');
  expect(wrapper.text()).toContain('192.168.1.15');
  wrapper.unmount();
});

it('pauses and resumes the sync, opens the client and quits from its menu', async () => {
  const wrapper = await mini();
  const choose = async (id: string) => {
    await wrapper.get('[data-testid="dropdown_toggle"]').trigger('click');
    await wrapper.get(`[data-testid="dropdown_${id}"]`).trigger('click');
    await flushPromises();
  };
  await choose('pause');
  expect(invoke).toHaveBeenCalledWith('set_paused', { paused: true });
  answer({
    get_state: snapshot({ paused: true, status: 'paused' }),
    get_history: [],
    set_paused: undefined,
  });
  await choose('pause');
  await choose('resume');
  expect(invoke).toHaveBeenCalledWith('set_paused', { paused: false });
  await choose('open');
  expect(invoke).toHaveBeenCalledWith('open_main', undefined);
  await choose('quit');
  expect(invoke).toHaveBeenCalledWith('quit_app', undefined);
  await wrapper.get('[data-testid="mini_open"]').trigger('click');
  await wrapper.get('[data-testid="mini_view"]').trigger('keydown', { key: 'Escape' });
  expect(invoke).toHaveBeenCalledWith('hide_mini', undefined);
  wrapper.unmount();
});

it('cannot pause before a NAS is connected and reports a failed command', async () => {
  const wrapper = await mini(snapshot({ link: null, status: 'unconfigured' }));
  await wrapper.get('[data-testid="dropdown_toggle"]').trigger('click');
  expect(wrapper.get('[data-testid="dropdown_pause"]').attributes('disabled')).toBeDefined();
  vi.mocked(invoke).mockRejectedValue('busy');
  await wrapper.get('[data-testid="dropdown_open"]').trigger('click');
  await wrapper.get('[data-testid="dropdown_toggle"]').trigger('click');
  await wrapper.get('[data-testid="dropdown_quit"]').trigger('click');
  await wrapper.get('[data-testid="mini_view"]').trigger('keydown', { key: 'Escape' });
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('quit_app', undefined);
  wrapper.unmount();
});
