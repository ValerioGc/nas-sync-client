<script setup lang="ts">

import { onMounted, ref } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { discoverNas, errorCode } from '@/services/client';
import type { ClientError, NasDevice } from '@/types';

import CommonSpinner from '@/components/common/CommonSpinner.vue';
import CommonModal from '@/components/common/CommonModal.vue';
import NasSetupNotice from '@/components/NasSetupNotice.vue';

import refreshIcon from '@/assets/icons/refresh.svg?no-inline';
import serverIcon from '@/assets/icons/server.svg?no-inline';

const emit = defineEmits<{
  close: [];
  select: [device: NasDevice];
}>();

const { t, errorText } = useI18n();
const devices = ref<NasDevice[]>([]);
const busy = ref(false);
const error = ref<ClientError | null>(null);

async function submit() {
  if (busy.value) {
    return;
  }

  busy.value = true;
  error.value = null;
  try {
    devices.value = await discoverNas();
  } catch (reason) {
    devices.value = [];
    error.value = errorCode(reason);
  } finally {
    busy.value = false;
  }
}

onMounted(submit);

</script>

<template>
  <CommonModal :title="t('searchTitle')" @close="emit('close')">
    <p class="body_copy description">{{ t('searchDescription') }}</p>

    <p v-if="error" class="error_message" role="alert">{{ errorText(error) }}</p>

    <section class="results" :aria-busy="busy">
      <output v-if="busy" class="empty_state search_progress">
        <CommonSpinner />
        <span>{{ t('searching') }}</span>
      </output>

      <ul v-else-if="devices.length" class="device_list">
        <li v-for="device in devices" :key="device.id" class="device_row" data-testid="nas_row">
          <span class="device_symbol">
            <span
              class="icon"
              :style="{ '--icon_url': `url(${serverIcon})` }"
              aria-hidden="true"
            ></span>
          </span>
          <span class="device_text">
            <strong>{{ device.name }}</strong>
            <small>
              {{ device.address }}
              <template v-if="device.configured">· {{ t('configured') }}</template>
            </small>
          </span>
          <button
            class="secondary_button"
            :disabled="!device.configured"
            :aria-describedby="!device.configured ? `setup_${device.id}` : undefined"
            @click="emit('select', device)"
          >
            {{ t('select') }}
          </button>
          <NasSetupNotice
            v-if="!device.configured"
            :id="`setup_${device.id}`"
            class="device_warning"
          />
        </li>
      </ul>

      <div v-else-if="!error" class="empty_state empty_results">
        <span
          class="icon"
          :style="{ '--icon_url': `url(${serverIcon})` }"
          aria-hidden="true"
        ></span>
        <h3>{{ t('noResults') }}</h3>
        <p class="hint">{{ t('noResultsDescription') }}</p>
      </div>
    </section>

    <div v-if="!busy" class="modal_actions">
      <button class="secondary_button" type="button" data-testid="search_submit" @click="submit">
        <span
          class="icon"
          :style="{ '--icon_url': `url(${refreshIcon})` }"
          aria-hidden="true"
        ></span>
        {{ t('searchAgain') }}
      </button>
    </div>
  </CommonModal>
</template>

<style scoped lang="scss">

.description {
  margin-bottom: $space_6;
}

.results {
  margin-top: $space_6;
}

.device_list {
  @extend %stack;
  gap: $space_2;
}

.device_row {
  display: grid;
  grid-template-columns: $control_size minmax(0, 1fr) auto;
  align-items: center;
  @include surface($background: var(--surface-soft));
  gap: $space_3;
  padding: $space_3 $space_4;
}

.device_warning {
  grid-column: 1 / -1;
  margin-bottom: 0;
}

.device_symbol {
  @extend %selected_control;
  display: inline-grid;
  place-items: center;
  @include square($control_size);
  border-radius: $radius_small;
}

.device_text {
  @extend %stack;
  min-width: 0;
  overflow-wrap: anywhere;

  small {
    @extend %muted_label;
  }
}

h3 {
  font-size: $font_body;
  font-weight: 600;
}

.empty_results,
.search_progress {
  min-height: 164px;
  padding: $space_6 $space_4;
}

.empty_results > .icon {
  @include square(28px);
  color: var(--muted);
  margin-bottom: $space_1;
}

.empty_results p {
  max-width: 400px;
}

.search_progress {
  color: var(--muted);
}

.error_message {
  margin-top: $space_4;
  margin-bottom: 0;
}

@media (max-height: 540px) {
  .description {
    margin-bottom: $space_4;
  }

  .results {
    margin-top: $space_4;
  }

  .empty_results,
  .search_progress {
    min-height: 100px;
    padding: $space_3;
    gap: $space_2;
  }

  .empty_results > .icon {
    display: none;
  }
}

</style>
