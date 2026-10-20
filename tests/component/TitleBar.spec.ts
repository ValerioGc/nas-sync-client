import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import TitleBar from '@/components/TitleBar.vue';
import { setLocale } from '@/i18n/i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  setLocale('en');
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

it('routes every window control and drag gesture to its native command', async () => {
  const wrapper = mount(TitleBar);
  await wrapper.find('button[aria-label="Minimize"]').trigger('click');
  expect(invoke).toHaveBeenLastCalledWith('minimize_window');
  await wrapper.find('button[aria-label="Maximize or restore"]').trigger('click');
  expect(invoke).toHaveBeenLastCalledWith('toggle_maximize_window');
  await wrapper.find('button[aria-label="Close"]').trigger('click');
  expect(invoke).toHaveBeenLastCalledWith('close_window');
  await wrapper.find('.titlebar_identity').trigger('mousedown', { button: 0 });
  expect(invoke).toHaveBeenLastCalledWith('start_window_dragging');
  await wrapper.find('.titlebar_identity').trigger('dblclick');
  expect(invoke).toHaveBeenLastCalledWith('toggle_maximize_window');
  expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  wrapper.unmount();
});

it('reports a native failure and clears the error after a successful retry', async () => {
  const wrapper = mount(TitleBar);
  vi.mocked(invoke).mockRejectedValueOnce(new Error('Native window unavailable'));
  await wrapper.find('button[aria-label="Minimize"]').trigger('click');
  await flushPromises();
  expect(wrapper.find('[role="alert"]').text()).toBe('Window command unavailable');
  await wrapper.find('button[aria-label="Minimize"]').trigger('click');
  await flushPromises();
  expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  wrapper.unmount();
});
