<script setup lang="ts">

import { ref } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { formatFingerprint } from '@/services/client';
import type { ClientError, LinkView, Preferences } from '@/types';

import CommonModal from '@/components/common/CommonModal.vue';
import CommonSwitch from '@/components/common/CommonSwitch.vue';
import ConfirmModal from '@/components/common/ConfirmModal.vue';

interface Props {
  link: LinkView;
  preferences: Preferences;
  busy: boolean;
  error: ClientError | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  move: [host: string, port: number];
  disconnect: [];
  save: [preferences: Preferences];
}>();

const { t, errorText } = useI18n();

const host = ref(props.link.host);
const port = ref(props.link.port);
const draft = ref<Preferences>({ ...props.preferences });
const confirming = ref(false);

const limit = (value: number) => Math.min(Math.max(Math.floor(Number(value) || 0), 0), 1_048_576);

function save() {
  emit('save', {
    ...draft.value,
    maxSendKbps: limit(draft.value.maxSendKbps),
    maxRecvKbps: limit(draft.value.maxRecvKbps),
  });
}

</script>

<template>
  <CommonModal :title="t('optionsTitle')" @close="emit('close')">
    <form id="options_form" class="options" @submit.prevent="save">
      <section class="options_section" aria-labelledby="general_title">
        <h3 id="general_title">{{ t('generalSection') }}</h3>
        <div class="options_card">
          <CommonSwitch
            v-model="draft.notifications"
            :label="t('notifications')"
            :hint="t('notificationsHint')"
            testid="options_notifications"
          />
          <CommonSwitch
            v-model="draft.autostart"
            :label="t('autostart')"
            testid="options_autostart"
          />
        </div>
      </section>

      <section class="options_section" aria-labelledby="bandwidth_title">
        <h3 id="bandwidth_title">{{ t('bandwidthSection') }}</h3>
        <div class="options_card">
          <div class="field_row">
            <label class="field field_wide">
              {{ t('maxSend') }}
              <input
                v-model.number="draft.maxSendKbps"
                name="max_send"
                type="number"
                min="0"
                data-testid="options_send"
              />
            </label>
            <label class="field field_wide">
              {{ t('maxRecv') }}
              <input v-model.number="draft.maxRecvKbps" name="max_recv" type="number" min="0" />
            </label>
          </div>
          <p class="hint">{{ t('bandwidthHint') }}</p>
        </div>
      </section>
    </form>

    <section class="options_section" aria-labelledby="connection_title">
      <h3 id="connection_title">{{ t('connectionSection') }}</h3>
      <div class="options_card">
        <dl class="link_details">
          <div>
            <dt>{{ t('computerName') }}</dt>
            <dd>{{ link.name }}</dd>
          </div>
          <div>
            <dt>{{ t('fingerprint') }}</dt>
            <dd class="fingerprint">{{ formatFingerprint(link.fingerprint) }}</dd>
          </div>
        </dl>
        <form class="field_row" @submit.prevent="emit('move', host, Number(port))">
          <label class="field field_wide">
            {{ t('nasAddress') }}
            <input v-model="host" name="nas_host" autocomplete="off" data-testid="options_host" />
          </label>
          <label class="field">
            {{ t('httpsPort') }}
            <input v-model.number="port" name="nas_port" type="number" min="1" max="65535" />
          </label>
          <button
            type="submit"
            class="secondary_button"
            :disabled="busy || !host.trim()"
            data-testid="options_move"
          >
            {{ t('changeAddress') }}
          </button>
        </form>
        <p class="hint">{{ t('moveHint') }}</p>
      </div>
    </section>

    <p v-if="error" class="error_message" role="alert">{{ errorText(error) }}</p>

    <div class="modal_actions">
      <button
        type="button"
        class="secondary_button danger_text"
        :disabled="busy"
        data-testid="options_disconnect"
        @click="confirming = true"
      >
        {{ t('disconnect') }}
      </button>
      <button
        type="submit"
        form="options_form"
        class="primary_button"
        :disabled="busy"
        data-testid="options_save"
      >
        {{ t('save') }}
      </button>
    </div>

    <ConfirmModal
      v-if="confirming"
      :title="t('disconnectTitle')"
      :description="t('disconnectDescription')"
      :action="t('disconnect')"
      :busy="busy"
      @close="confirming = false"
      @confirm="emit('disconnect')"
    />
  </CommonModal>
</template>

<style scoped lang="scss">

.options {
  @extend %stack;
  gap: $space_6;
}

.options_section {
  @extend %stack;
  gap: $space_2;

  & + .options_section {
    margin-top: $space_6;
  }

  h3 {
    @extend %muted_label;
    font-size: $font_small;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
}

// The settings of a section in one card, each on its own line.
.options_card {
  @include surface($background: var(--surface-soft), $radius: $radius_medium);
  @extend %stack;
  gap: $space_4;
  padding: $space_4;
}

.link_details {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: $space_3;
  margin: 0;

  div {
    @extend %stack;
    gap: $space_1;
    min-width: 0;
  }

  dt {
    @extend %muted_label;
  }

  dd {
    margin: 0;
  }
}

.fingerprint {
  font-family: $font_mono;
  font-size: $font_small;
  overflow-wrap: anywhere;
}

.field_row {
  @extend %fx_row;
  align-items: flex-end;
  gap: $space_3;

  .field {
    width: $port_field_width;
  }

  .field_wide {
    flex: 1;
  }
}

.error_message {
  margin: $space_4 0 0;
}

.modal_actions {
  margin-top: $space_6;

  // Disconnecting stays apart from saving, at the other end.
  .danger_text {
    margin-right: auto;
  }
}

@include compact {
  .link_details {
    grid-template-columns: minmax(0, 1fr);
  }

  .field_row {
    flex-direction: column;
    align-items: stretch;

    .field {
      width: 100%;
    }
  }
}

</style>
