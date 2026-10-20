import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import ConfirmModal from '@/components/common/ConfirmModal.vue';
import { setLocale } from '@/i18n/i18n';

beforeEach(() => setLocale('en'));

it('asks before an action and can be cancelled', async () => {
  const wrapper = mount(ConfirmModal, {
    props: { title: 'Sync removal', description: 'The files stay', action: 'Remove', busy: false },
  });
  await flushPromises();
  expect(document.body.textContent).toContain('The files stay');
  document.body.querySelector<HTMLButtonElement>('[data-testid="confirm_action"]')?.click();
  expect(wrapper.emitted('confirm')).toHaveLength(1);
  const cancel = [...document.body.querySelectorAll('button')].find(
    (button) => button.textContent?.trim() === 'Cancel',
  );
  cancel?.click();
  document.body.querySelector('dialog')?.click();
  expect(wrapper.emitted('close')).toHaveLength(2);
  wrapper.unmount();
});
