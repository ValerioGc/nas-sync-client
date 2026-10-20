import { mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import HistoryList from '@/components/HistoryList.vue';
import { setLocale } from '@/i18n/i18n';
import { entry, sync } from '@tests/fixtures';

beforeEach(() => setLocale('en'));

it('lists the updated files with their sync, action and direction', () => {
  const wrapper = mount(HistoryList, {
    props: {
      entries: [
        entry(),
        entry({
          path: 'a.sync-conflict-1.jpg',
          direction: 'download',
          action: 'added',
          conflict: true,
        }),
        entry({ path: 'big.iso', action: 'failed', folder: 'gone' }),
      ],
      syncs: [sync()],
    },
  });
  const rows = wrapper.findAll('[data-testid="history_entry"]');
  expect(rows).toHaveLength(3);
  expect(rows[0]?.text()).toContain('report.pdf');
  expect(rows[0]?.text()).toContain('Photos · Modified');
  expect(rows[1]?.text()).toContain('Conflict copy');
  expect(rows[1]?.classes()).toContain('entry_problem');
  expect(rows[2]?.text()).toContain('gone · Not synced');
  expect(rows[0]?.find('.entry_icon').attributes('title')).toBe('To the NAS');
  expect(rows[1]?.find('.entry_icon').attributes('title')).toBe('To the computer');
});

it('says when no file was updated', () => {
  const wrapper = mount(HistoryList, { props: { entries: [], syncs: [], compact: true } });
  expect(wrapper.get('[data-testid="history_empty"]').text()).toBe('No updated files');
});
