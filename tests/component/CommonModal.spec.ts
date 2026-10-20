import { flushPromises, mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import CommonModal from '@/components/common/CommonModal.vue';

it('dismisses only backdrop clicks while keeping clicks inside the dialog open', async () => {
  const wrapper = mount(CommonModal, {
    props: { title: 'Search' },
    slots: { default: '<p>Content</p>' },
  });

  await flushPromises();
  document.body.querySelector<HTMLParagraphElement>('dialog p')?.click();
  expect(wrapper.emitted('close')).toBeUndefined();
  document.body.querySelector('dialog')?.click();
  expect(wrapper.emitted('close')).toHaveLength(1);
  wrapper.unmount();
});
