import { mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import StatusBadge from '@/components/StatusBadge.vue';
import { setLocale } from '@/i18n/i18n';
import type { Status } from '@/types';

beforeEach(() => setLocale('en'));

it('names every state of the client with its own look', () => {
  const states: [Status, string][] = [
    ['synced', 'Synced'],
    ['syncing', 'Syncing'],
    ['paused', 'Sync paused'],
    ['disconnected', 'Not connected'],
    ['error', 'Sync errors'],
    ['unconfigured', 'No NAS connected'],
  ];
  for (const [status, label] of states) {
    const wrapper = mount(StatusBadge, { props: { status } });
    expect(wrapper.text()).toBe(label);
    expect(wrapper.classes()).toContain(`status_${status}`);
    wrapper.unmount();
  }
});
