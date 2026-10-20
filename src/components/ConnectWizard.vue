<script setup lang="ts">

import { computed, onMounted, ref } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { connectNas, defaultName, errorCode, formatFingerprint, probeNas } from '@/services/client';
import type { ClientError, NasDevice, WizardStep } from '@/types';

import CommonSpinner from '@/components/common/CommonSpinner.vue';
import SearchNasModal from '@/components/SearchNasModal.vue';
import NasSetupNotice from '@/components/NasSetupNotice.vue';

import checkIcon from '@/assets/icons/check.svg?no-inline';
import searchIcon from '@/assets/icons/search.svg?no-inline';
import serverIcon from '@/assets/icons/server.svg?no-inline';

const emit = defineEmits<{ connected: [] }>();

const { t, errorText } = useI18n();

const STEPS: WizardStep[] = ['address', 'trust', 'account'];
const STEP_LABELS = { address: 'stepAddress', trust: 'stepTrust', account: 'stepAccount' } as const;

const step = ref<WizardStep>('address');
const host = ref('');
const port = ref(443);
const fingerprint = ref('');
const username = ref('');
const password = ref('');
const name = ref('');
const busy = ref(false);
const error = ref<ClientError | null>(null);
const searching = ref(false);

const addressReady = computed(() => host.value.trim().length > 0 && port.value > 0);
const accountReady = computed(
  () => username.value.trim() && password.value && name.value.trim().length > 0,
);

onMounted(async () => {
  name.value = await defaultName().catch(() => '');
});

async function attempt(action: () => Promise<void>) {
  busy.value = true;
  error.value = null;
  try {
    await action();
  } catch (reason) {
    error.value = errorCode(reason);
  } finally {
    busy.value = false;
  }
}

const test = () =>
  attempt(async () => {
    fingerprint.value = (await probeNas(host.value, Number(port.value))).fingerprint;
    step.value = 'trust';
  });

const connect = () =>
  attempt(async () => {
    await connectNas({
      host: host.value,
      port: Number(port.value),
      fingerprint: fingerprint.value,
      username: username.value.trim(),
      password: password.value,
      name: name.value.trim(),
    });
    password.value = '';
    emit('connected');
  });

/** Selects a discovered NAS address and its advertised HTTPS port. */
function choose(device: NasDevice) {
  host.value = device.address;
  port.value = device.secure ? device.port : 443;
  searching.value = false;
}

function back() {
  error.value = null;
  step.value = step.value === 'account' ? 'trust' : 'address';
}

</script>

<template>
  <section class="wizard" aria-labelledby="wizard_title" data-testid="connect_wizard">
    <header class="wizard_header">
      <span class="wizard_symbol">
        <span
          class="icon"
          :style="{ '--icon_url': `url(${serverIcon})` }"
          aria-hidden="true"
        ></span>
      </span>
      <div>
        <h2 id="wizard_title">{{ t('wizardTitle') }}</h2>
        <p class="body_copy">{{ t('wizardDescription') }}</p>
      </div>
    </header>

    <ol class="wizard_steps">
      <li
        v-for="(item, index) in STEPS"
        :key="item"
        :class="{ step_current: item === step, step_done: STEPS.indexOf(step) > index }"
        :aria-current="item === step ? 'step' : undefined"
      >
        <span class="step_number">{{ index + 1 }}</span>
        {{ t(STEP_LABELS[item]) }}
      </li>
    </ol>

    <form v-if="step === 'address'" class="wizard_body" @submit.prevent="test">
      <div class="field_row">
        <label class="field field_wide">
          {{ t('nasAddress') }}
          <input
            v-model="host"
            autocomplete="off"
            placeholder="nas.local"
            :disabled="busy"
            data-testid="wizard_host"
          />
          <small class="hint">{{ t('nasAddressHint') }}</small>
        </label>
        <label class="field">
          {{ t('httpsPort') }}
          <input
            v-model.number="port"
            type="number"
            min="1"
            max="65535"
            :disabled="busy"
            data-testid="wizard_port"
          />
        </label>
      </div>
      <div class="modal_actions">
        <button type="button" class="secondary_button" :disabled="busy" @click="searching = true">
          <span
            class="icon"
            :style="{ '--icon_url': `url(${searchIcon})` }"
            aria-hidden="true"
          ></span>
          {{ t('findNas') }}
        </button>
        <button
          type="submit"
          class="primary_button"
          :disabled="busy || !addressReady"
          :aria-busy="busy"
          data-testid="wizard_test"
        >
          <CommonSpinner v-if="busy" />
          <span
            v-else
            class="icon"
            :style="{ '--icon_url': `url(${checkIcon})` }"
            aria-hidden="true"
          ></span>
          {{ t('testLink') }}
        </button>
      </div>
    </form>

    <div v-else-if="step === 'trust'" class="wizard_body">
      <h3>{{ t('trustTitle') }}</h3>
      <p class="body_copy">{{ t('trustDescription') }}</p>
      <dl class="trust_details">
        <dt>{{ t('nasAddress') }}</dt>
        <dd>{{ host }}:{{ port }}</dd>
        <dt>{{ t('fingerprint') }}</dt>
        <dd class="fingerprint" data-testid="wizard_fingerprint">
          {{ formatFingerprint(fingerprint) }}
        </dd>
      </dl>
      <div class="modal_actions">
        <button class="secondary_button" @click="back">{{ t('back') }}</button>
        <button class="primary_button" data-testid="wizard_trust" @click="step = 'account'">
          {{ t('trustConfirm') }}
        </button>
      </div>
    </div>

    <form v-else class="wizard_body" @submit.prevent="connect">
      <label class="field">
        {{ t('username') }}
        <input
          v-model="username"
          autocomplete="username"
          :disabled="busy"
          data-testid="wizard_username"
        />
      </label>
      <label class="field">
        {{ t('password') }}
        <input
          v-model="password"
          type="password"
          autocomplete="current-password"
          :disabled="busy"
          data-testid="wizard_password"
        />
      </label>
      <label class="field">
        {{ t('computerName') }}
        <input v-model="name" maxlength="60" :disabled="busy" data-testid="wizard_name" />
        <small class="hint">{{ t('computerNameHint') }}</small>
      </label>
      <div class="modal_actions">
        <button type="button" class="secondary_button" :disabled="busy" @click="back">
          {{ t('back') }}
        </button>
        <button
          type="submit"
          class="primary_button"
          :disabled="busy || !accountReady"
          data-testid="wizard_connect"
        >
          <CommonSpinner v-if="busy" />
          {{ busy ? t('connecting') : t('connect') }}
        </button>
      </div>
    </form>

    <NasSetupNotice
      v-if="error === 'nas_not_configured'"
      class="wizard_notice"
      role="alert"
      data-testid="wizard_error"
    />
    <p v-else-if="error" class="error_message" role="alert" data-testid="wizard_error">
      {{ errorText(error) }}
    </p>

    <SearchNasModal v-if="searching" @close="searching = false" @select="choose" />
  </section>
</template>

<style scoped lang="scss">

.wizard {
  @include surface($radius: $radius_large, $shadow: var(--panel-shadow));
  width: 100%;
  max-width: $wizard_width;
  margin-inline: auto;
  padding: $space_8;
}

.wizard_header {
  @extend %fx_row;
  gap: $space_4;

  h2 {
    font-size: $font_title;
    font-weight: 600;
    margin-bottom: $space_1;
  }
}

.wizard_symbol {
  @extend %selected_control;
  display: grid;
  place-items: center;
  @include square(48px);
  flex-shrink: 0;
  border-radius: $radius_medium;
}

.wizard_steps {
  @extend %fx_row;
  gap: $space_4;
  margin: $space_6 0;
  font-size: $font_small;
  color: var(--muted);

  li {
    @extend %fx_base;
    gap: $space_2;
  }
}

.step_number {
  display: inline-grid;
  place-items: center;
  @include square(24px);
  border-radius: $radius_round;
  border: $border_width solid var(--border);
}

.step_current {
  color: var(--accent);
  font-weight: 600;

  .step_number {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
}

.step_done .step_number {
  color: var(--success);
  border-color: var(--success);
}

.wizard_body {
  @extend %stack;
  gap: $space_4;

  h3 {
    font-size: $font_body;
    font-weight: 600;
  }
}

.field_row {
  @extend %fx_row;
  gap: $space_4;

  .field {
    width: $port_field_width;
  }

  .field_wide {
    flex: 1;
  }
}

.trust_details {
  @extend %stack;
  gap: $space_1;

  dt {
    @extend %muted_label;
    margin-top: $space_2;
  }
}

.fingerprint {
  font-family: $font_mono;
  font-size: $font_small;
  overflow-wrap: anywhere;
}

.error_message,
.wizard_notice {
  margin: $space_4 0 0;
}

@include compact {
  .wizard {
    padding: $space_5;
  }

  .field_row {
    flex-direction: column;

    .field {
      width: 100%;
    }
  }
}

</style>
