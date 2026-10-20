import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import OptionsModal from '@/components/OptionsModal.vue';
import { setLocale } from '@/i18n/i18n';
import { snapshot } from '@tests/fixtures';

beforeEach(() => setLocale('en'));

const state = snapshot();
const props = { link: state.link!, preferences: state.preferences, busy: false, error: null };
const find = (testid: string) =>
  document.body.querySelector<HTMLInputElement>(`[data-testid="${testid}"]`);

it('moves the link to a new address of the same NAS', async () => {
  const wrapper = mount(OptionsModal, { props });
  await flushPromises();
  expect(document.body.textContent).toContain('AB:AB');
  const host = find('options_host')!;
  host.value = 'nas.local';
  host.dispatchEvent(new Event('input'));
  find('options_move')?.closest('form')?.dispatchEvent(new Event('submit'));
  expect(wrapper.emitted('move')).toEqual([['nas.local', 443]]);
  wrapper.unmount();
});

it('saves the preferences within their limits', async () => {
  const wrapper = mount(OptionsModal, { props });
  await flushPromises();
  find('options_notifications')?.click();
  find('options_autostart')?.click();
  const send = find('options_send')!;
  send.value = '99999999';
  send.dispatchEvent(new Event('input'));
  await flushPromises();

  expect(find('options_save')?.getAttribute('form')).toBe('options_form');
  document.getElementById('options_form')?.dispatchEvent(new Event('submit'));
  expect(wrapper.emitted('save')).toEqual([
    [{ notifications: false, autostart: true, maxSendKbps: 1_048_576, maxRecvKbps: 0 }],
  ]);
  wrapper.unmount();
});

it('disconnects only once confirmed, and shows a refusal', async () => {
  const wrapper = mount(OptionsModal, { props: { ...props, error: 'trust_changed' } });
  await flushPromises();
  expect(document.body.textContent).toContain('does not match the confirmed one');
  find('options_disconnect')?.click();
  await flushPromises();
  expect(document.body.textContent).toContain('The syncs of this computer end');
  find('confirm_action')?.click();
  expect(wrapper.emitted('disconnect')).toHaveLength(1);
  const cancel = [...document.body.querySelectorAll('button')].find(
    (button) => button.textContent?.trim() === 'Cancel',
  );
  cancel?.click();
  await flushPromises();
  expect(document.body.textContent).not.toContain('The syncs of this computer end');
  wrapper.unmount();
});
