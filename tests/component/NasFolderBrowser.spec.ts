import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import NasFolderBrowser from '@/components/NasFolderBrowser.vue';
import { setLocale } from '@/i18n/i18n';
import { answer } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const tree: Record<string, { name: string; path: string }[]> = {
  '': [{ name: 'Data', path: 'v-1' }],
  'v-1': [{ name: 'Photos', path: 'v-1/Photos' }],
  'v-1/Photos': [],
};

beforeEach(() => {
  setLocale('en');
  vi.mocked(invoke).mockReset();
  answer({ nas_folders: ({ path }: { path: string }) => tree[path] });
});

it('walks the folders of the NAS from its disks and picks the one open', async () => {
  const wrapper = mount(NasFolderBrowser, {
    props: {
      modelValue: '',
      'onUpdate:modelValue': (value: string) => wrapper.setProps({ modelValue: value }),
    },
  });
  await flushPromises();
  expect(wrapper.get('[data-testid="nas_selected"]').text()).toBe('Disks');
  await wrapper.get('[data-testid="nas_folder"]').trigger('click');
  await flushPromises();
  await wrapper.get('[data-testid="nas_folder"]').trigger('click');
  await flushPromises();
  expect(wrapper.props('modelValue')).toBe('v-1/Photos');
  expect(wrapper.get('[data-testid="nas_selected"]').text()).toBe('Data / Photos');
  expect(wrapper.text()).toContain('No subfolders');
  await wrapper.get('button').trigger('click');
  await flushPromises();
  expect(wrapper.props('modelValue')).toBe('v-1');
  await wrapper.get('button').trigger('click');
  await flushPromises();
  expect(wrapper.props('modelValue')).toBe('');
});

it('says why the folders cannot be read', async () => {
  vi.mocked(invoke).mockRejectedValue('nas_unreachable');
  const wrapper = mount(NasFolderBrowser, { props: { modelValue: '' } });
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toContain('NAS unreachable');
});
