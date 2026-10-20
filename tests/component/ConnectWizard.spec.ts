import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import ConnectWizard from '@/components/ConnectWizard.vue';
import { setLocale } from '@/i18n/i18n';
import { answer, nas } from '@tests/fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

beforeEach(() => {
  setLocale('en');
  vi.mocked(invoke).mockReset();
  answer({
    default_name: 'studio-pc',
    probe_nas: { fingerprint: 'ab'.repeat(32) },
    discover_nas: [nas({ address: '192.168.1.20', port: 8443 })],
    connect_nas: undefined,
  });
});

it('connects the NAS after the test of the link and the confirmed fingerprint', async () => {
  const wrapper = mount(ConnectWizard);
  await flushPromises();
  expect(wrapper.get('[data-testid="wizard_test"]').attributes('disabled')).toBeDefined();
  await wrapper.get('[data-testid="wizard_host"]').setValue('nas.local');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('probe_nas', { host: 'nas.local', port: 443 });
  expect(wrapper.get('[data-testid="wizard_fingerprint"]').text()).toBe(
    Array(32).fill('AB').join(':'),
  );
  await wrapper.get('[data-testid="wizard_trust"]').trigger('click');
  expect(wrapper.get<HTMLInputElement>('[data-testid="wizard_name"]').element.value).toBe(
    'studio-pc',
  );
  await wrapper.get('[data-testid="wizard_username"]').setValue('admin');
  await wrapper.get('[data-testid="wizard_password"]').setValue('secret');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(invoke).toHaveBeenCalledWith('connect_nas', {
    host: 'nas.local',
    port: 443,
    fingerprint: 'ab'.repeat(32),
    username: 'admin',
    password: 'secret',
    name: 'studio-pc',
  });
  expect(wrapper.emitted('connected')).toHaveLength(1);
  wrapper.unmount();
});

it('says why a link fails and goes back a step', async () => {
  const wrapper = mount(ConnectWizard);
  await flushPromises();
  vi.mocked(invoke).mockRejectedValueOnce('not_a_nas');
  await wrapper.get('[data-testid="wizard_host"]').setValue('printer');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(wrapper.get('[data-testid="wizard_error"]').text()).toContain('does not answer as a NAS');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  await wrapper.get('[data-testid="wizard_trust"]').trigger('click');
  await wrapper.get('[data-testid="wizard_username"]').setValue('admin');
  await wrapper.get('[data-testid="wizard_password"]').setValue('bad');
  vi.mocked(invoke).mockRejectedValueOnce('invalid_credentials');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(wrapper.get('[data-testid="wizard_error"]').text()).toBe('Wrong username or password');
  const back = () => wrapper.findAll('button').find((button) => button.text() === 'Back');
  await back()?.trigger('click');
  expect(wrapper.find('[data-testid="wizard_fingerprint"]').exists()).toBe(true);
  await back()?.trigger('click');
  expect(wrapper.find('[data-testid="wizard_host"]').exists()).toBe(true);
  wrapper.unmount();
});

it('fills the address with a NAS found on the network', async () => {
  const wrapper = mount(ConnectWizard);
  await flushPromises();
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Search')
    ?.trigger('click');
  await flushPromises();
  document.body.querySelector<HTMLButtonElement>('[data-testid="nas_row"] button')?.click();
  await flushPromises();
  expect(wrapper.get<HTMLInputElement>('[data-testid="wizard_host"]').element.value).toBe(
    '192.168.1.20',
  );
  expect(wrapper.get<HTMLInputElement>('[data-testid="wizard_port"]').element.value).toBe('8443');
  expect(document.body.querySelector('dialog')).toBeNull();
  wrapper.unmount();
});

it('warns that a NAS found by its IP needs setup before creating syncs', async () => {
  const wrapper = mount(ConnectWizard);
  await flushPromises();
  vi.mocked(invoke).mockRejectedValueOnce('nas_unreachable');
  vi.mocked(invoke).mockResolvedValueOnce([nas({ configured: false })]);
  await wrapper.get('[data-testid="wizard_host"]').setValue('192.168.1.15');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  const warning = wrapper.get('[data-testid="wizard_error"]');
  expect(warning.classes()).toContain('warning_message');
  expect(warning.text()).toContain('The NAS was detected successfully');
  expect(warning.text()).toContain('connect this computer and synchronize folders');
  expect(wrapper.find('[data-testid="wizard_fingerprint"]').exists()).toBe(false);
  expect(wrapper.get('[data-testid="wizard_test"]').attributes('disabled')).toBeUndefined();
  wrapper.unmount();
});

it('keeps the HTTPS port of a NAS that announces a plain web page, and copes without a name', async () => {
  answer({
    default_name: () => Promise.reject(new Error('no name')),
    discover_nas: [nas({ secure: false, port: 80 })],
  });
  const wrapper = mount(ConnectWizard);
  await flushPromises();
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Search')
    ?.trigger('click');
  await flushPromises();
  document.body.querySelector<HTMLButtonElement>('[data-testid="nas_row"] button')?.click();
  await flushPromises();
  expect(wrapper.get<HTMLInputElement>('[data-testid="wizard_port"]').element.value).toBe('443');
  document.body.querySelector<HTMLButtonElement>('dialog button[aria-label="Close"]')?.click();
  wrapper.unmount();
});
