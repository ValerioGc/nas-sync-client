import { ref } from 'vue';
import en from '@/i18n/lang/en';
import it from '@/i18n/lang/it';
import type { ClientError, Locale, MessageKey } from '@/types';

const messages = { en, it } as const;
const locale = ref<Locale>('it');

export function setLocale(nextLocale: Locale) {
  locale.value = nextLocale;
  document.documentElement.lang = nextLocale;
}

export function t(key: MessageKey) {
  return messages[locale.value][key];
}

const errorKeys: Record<ClientError, MessageKey> = {
  invalid_address: 'errorInvalidAddress',
  network_unavailable: 'errorNetworkUnavailable',
  nas_unreachable: 'errorNasUnreachable',
  nas_not_configured: 'errorNasNotConfigured',
  not_a_nas: 'errorNotANas',
  trust_changed: 'errorTrustChanged',
  invalid_credentials: 'errorInvalidCredentials',
  too_many_attempts: 'errorTooManyAttempts',
  security_required: 'errorSecurityRequired',
  not_paired: 'errorNotPaired',
  app_missing: 'errorAppMissing',
  engine_unavailable: 'errorEngineUnavailable',
  storage_unavailable: 'errorStorageUnavailable',
  secret_unavailable: 'errorSecretUnavailable',
  invalid_sync: 'errorInvalidSync',
  folder_taken: 'errorFolderTaken',
  not_connected: 'errorNotConnected',
  busy: 'errorBusy',
};

export function useI18n() {
  return { locale, setLocale, t, errorText: (code: ClientError) => t(errorKeys[code]) };
}
