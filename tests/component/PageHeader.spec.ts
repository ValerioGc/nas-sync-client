import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PageHeader from '@/components/layout/PageHeader.vue';
import { setTheme } from '@/composables/useTheme';
import { setLocale } from '@/i18n/i18n';
import italian from '@/i18n/lang/it';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  setLocale('en');
  setTheme('dark');
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

async function choose(wrapper: VueWrapper, id: string, label: string) {
  await wrapper.get(`#${id}`).trigger('click');
  const option = wrapper.findAll('[role="menuitemradio"]').find((item) => item.text() === label);
  expect(option).toBeDefined();
  await option?.trigger('click');
}

it('applies the selected theme and language while keeping accessible names on icon controls', async () => {
  const wrapper = mount(PageHeader, { attachTo: document.body });
  expect(wrapper.find('label').exists()).toBe(false);
  expect(wrapper.find('#theme').attributes('aria-label')).toBe('Theme: Dark');
  expect(wrapper.find('#language').attributes('aria-label')).toBe('Language: English');
  await choose(wrapper, 'theme', 'Light');
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(document.documentElement.style.colorScheme).toBe('light');
  await choose(wrapper, 'theme', 'Dark');
  expect(document.documentElement.dataset.theme).toBe('dark');
  await choose(wrapper, 'language', 'Italian');
  expect(document.documentElement.lang).toBe('it');
  expect(wrapper.find('#language').attributes('aria-label')).toBe(
    `${italian.language}: ${italian.italian}`,
  );
  await choose(wrapper, 'language', 'Inglese');
  expect(document.documentElement.lang).toBe('en');
  expect(wrapper.find('#language').attributes('aria-label')).toBe('Language: English');
  expect(invoke).toHaveBeenLastCalledWith('set_appearance', {
    appearance: { locale: 'en', theme: 'dark' },
  });
  vi.mocked(invoke).mockRejectedValue('storage_unavailable');
  await choose(wrapper, 'theme', 'Light');
  await flushPromises();
  expect(document.documentElement.dataset.theme).toBe('light');
  wrapper.unmount();
});

it('keeps the selector and the document in sync with programmatic theme changes', async () => {
  const wrapper = mount(PageHeader, { attachTo: document.body });
  setTheme('light');
  await flushPromises();
  expect(wrapper.get('#theme').text()).toBe('Light');
  expect(document.documentElement.dataset.theme).toBe('light');
  setTheme('dark');
  await flushPromises();
  expect(wrapper.get('#theme').text()).toBe('Dark');
  expect(document.documentElement.dataset.theme).toBe('dark');
  wrapper.unmount();
});

it('supports keyboard selection, Escape, focus restoration and outside dismissal', async () => {
  const wrapper = mount(PageHeader, { attachTo: document.body });
  const trigger = wrapper.get('#theme');
  await trigger.trigger('keydown', { key: 'ArrowDown' });
  expect(trigger.attributes('aria-expanded')).toBe('true');
  let options = wrapper.findAll('[role="menuitemradio"]');
  expect(document.activeElement).toBe(options[0]?.element);
  expect(options[0]?.attributes('aria-checked')).toBe('true');
  await options[0]?.trigger('keydown', { key: 'End' });
  expect(document.activeElement).toBe(options[1]?.element);
  await options[1]?.trigger('click');
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(document.activeElement).toBe(trigger.element);
  expect(trigger.attributes('aria-expanded')).toBe('false');
  await trigger.trigger('keydown', { key: 'ArrowUp' });
  options = wrapper.findAll('[role="menuitemradio"]');
  expect(document.activeElement).toBe(options[1]?.element);
  await options[1]?.trigger('keydown', { key: 'ArrowDown' });
  expect(document.activeElement).toBe(options[0]?.element);
  await options[0]?.trigger('keydown', { key: 'Home' });
  expect(document.activeElement).toBe(options[0]?.element);
  await options[0]?.trigger('keydown', { key: 'Escape' });
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  expect(document.activeElement).toBe(trigger.element);
  await trigger.trigger('click');
  (wrapper.get('#language').element as HTMLButtonElement).focus();
  await flushPromises();
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  await trigger.trigger('click');
  document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }));
  await flushPromises();
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  wrapper.unmount();
});
