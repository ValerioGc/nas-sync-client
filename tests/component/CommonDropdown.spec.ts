import { mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import CommonDropdown from '@/components/common/CommonDropdown.vue';

const items = [
  { id: 'pause', label: 'Pause sync', icon: 'pause.svg' },
  { id: 'off', label: 'Unavailable', icon: 'off.svg', disabled: true },
];

it('opens a menu of actions and closes on a choice, outside or with Escape', async () => {
  const wrapper = mount(CommonDropdown, {
    props: { label: 'Menu', items },
    attachTo: document.body,
  });
  const toggle = wrapper.get('[data-testid="dropdown_toggle"]');
  expect(toggle.attributes('aria-label')).toBe('Menu');
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);

  await toggle.trigger('click');
  expect(toggle.attributes('aria-expanded')).toBe('true');
  await wrapper.get('[data-testid="dropdown_off"]').trigger('click');
  expect(wrapper.emitted('select')).toBeUndefined();
  await wrapper.get('[data-testid="dropdown_pause"]').trigger('click');
  expect(wrapper.emitted('select')).toEqual([['pause']]);
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);

  await toggle.trigger('click');
  await wrapper.get('[data-testid="dropdown_pause"]').trigger('mousedown');
  expect(wrapper.find('[role="menu"]').exists()).toBe(true);
  document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
  await wrapper.vm.$nextTick();
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);

  await toggle.trigger('click');
  await wrapper.get('.dropdown').trigger('keydown', { key: 'Escape' });
  expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  wrapper.unmount();
});
