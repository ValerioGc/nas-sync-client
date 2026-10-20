import { mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import SyncList from '@/components/SyncList.vue';
import { setLocale } from '@/i18n/i18n';
import { sync } from '@tests/fixtures';

beforeEach(() => setLocale('en'));

it('shows each sync with its folders, mode, state and progress', () => {
  const wrapper = mount(SyncList, {
    props: {
      busy: false,
      syncs: [
        sync(),
        sync({
          id: 'f-2',
          name: 'Docs',
          state: 'syncing',
          progress: 42.7,
          mode: 'nas-to-pc',
          lastSync: null,
        }),
        sync({ id: 'f-3', name: 'Old', state: 'revoked' }),
      ],
    },
  });
  const rows = wrapper.findAll('[data-testid="sync_row"]');
  expect(rows[0]?.text()).toContain('/home/me/Photos');
  expect(rows[0]?.text()).toContain('Data/Photos');
  expect(rows[0]?.text()).toContain('Two-way');
  expect(rows[1]?.text()).toContain('Syncing 42%');
  expect(rows[1]?.text()).toContain('NAS to computer');
  expect(rows[1]?.text()).toContain('Never');
  expect(rows[2]?.find('button[aria-label="Pause Old"]').attributes('disabled')).toBeDefined();
});

it('pauses, resumes, edits and removes a sync', async () => {
  const paused = sync({ id: 'f-2', name: 'Docs', state: 'paused' });
  const wrapper = mount(SyncList, { props: { busy: false, syncs: [sync(), paused] } });
  await wrapper.get('button[aria-label="Pause Photos"]').trigger('click');
  await wrapper.get('button[aria-label="Resume Docs"]').trigger('click');
  await wrapper.get('button[aria-label="Edit Photos"]').trigger('click');
  await wrapper.get('button[aria-label="Remove Docs"]').trigger('click');
  expect(wrapper.emitted('pause')).toEqual([
    [sync(), true],
    [paused, false],
  ]);
  expect(wrapper.emitted('edit')).toEqual([[sync()]]);
  expect(wrapper.emitted('remove')).toEqual([[paused]]);
});
