import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import SyncRuleModal from '@/components/SyncRuleModal.vue';
import { setLocale } from '@/i18n/i18n';
import { answer, sync } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  setLocale('en');
  vi.mocked(invoke).mockReset();
  answer({
    pick_folder: '/home/me/Music',
    nas_folders: ({ path }: { path: string }) =>
      path ? [{ name: 'Music', path: 'v-1/Music' }] : [{ name: 'Data', path: 'v-1' }],
  });
});

const button = (testid: string) =>
  document.body.querySelector<HTMLElement>(`[data-testid="${testid}"]`);

it('creates a sync from a folder of the computer and a folder of a disk of the NAS', async () => {
  const wrapper = mount(SyncRuleModal, { props: { busy: false, error: null } });
  await flushPromises();
  expect((button('rule_save') as HTMLButtonElement).disabled).toBe(true);
  button('rule_browse')?.click();
  await flushPromises();
  expect(button('rule_pc_path')?.textContent).toBe('/home/me/Music');
  expect((button('rule_name') as HTMLInputElement).value).toBe('Music');
  button('nas_folder')?.click();
  await flushPromises();
  expect((button('rule_save') as HTMLButtonElement).disabled).toBe(true);
  button('nas_folder')?.click();
  await flushPromises();
  document.body.querySelector<HTMLInputElement>('input[value="pc-to-nas"]')?.click();
  await flushPromises();
  document.body.querySelector('form')?.dispatchEvent(new Event('submit'));
  expect(wrapper.emitted('create')).toEqual([
    [{ name: 'Music', nasPath: 'v-1/Music', pcPath: '/home/me/Music', mode: 'pc-to-nas' }],
  ]);
  wrapper.unmount();
});

it('changes a sync but never its folder of the NAS, and shows a refusal', async () => {
  const wrapper = mount(SyncRuleModal, {
    props: { sync: sync(), busy: false, error: 'folder_taken' },
  });
  await flushPromises();
  expect(document.body.textContent).toContain('Data/Photos');
  expect(button('nas_browser')).toBeNull();
  expect(document.body.querySelector('[role="alert"]')?.textContent).toContain(
    'Folder already synced',
  );
  vi.mocked(invoke).mockResolvedValue(null);
  button('rule_browse')?.click();
  await flushPromises();
  expect(button('rule_pc_path')?.textContent).toBe('/home/me/Photos');
  const name = button('rule_name') as HTMLInputElement;
  name.value = 'Pictures';
  name.dispatchEvent(new Event('input'));
  document.body.querySelector('form')?.dispatchEvent(new Event('submit'));
  expect(wrapper.emitted('change')).toEqual([
    ['f-1', { name: 'Pictures', pcPath: '/home/me/Photos', mode: 'two-way' }],
  ]);
  const cancel = [...document.body.querySelectorAll('button')].find(
    (item) => item.textContent?.trim() === 'Cancel',
  );
  cancel?.click();
  expect(wrapper.emitted('close')).toHaveLength(1);
  wrapper.unmount();
});

it('saves nothing while busy', async () => {
  const wrapper = mount(SyncRuleModal, { props: { sync: sync(), busy: true, error: null } });
  await flushPromises();
  document.body.querySelector('form')?.dispatchEvent(new Event('submit'));
  expect(wrapper.emitted('change')).toBeUndefined();
  wrapper.unmount();
});
