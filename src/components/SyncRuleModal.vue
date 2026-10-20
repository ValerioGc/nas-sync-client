<script setup lang="ts">

import { computed, ref } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { pickFolder } from '@/services/client';
import type { ClientError, NewSync, SyncChange, SyncMode, SyncView } from '@/types';
import CommonModal from '@/components/common/CommonModal.vue';
import NasFolderBrowser from '@/components/NasFolderBrowser.vue';
import folderIcon from '@/assets/icons/folder.svg?no-inline';

interface Props {
  sync?: SyncView | null;
  busy: boolean;
  error: ClientError | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  create: [sync: NewSync];
  change: [id: string, change: SyncChange];
}>();

const { t, errorText } = useI18n();

const MODES: SyncMode[] = ['two-way', 'pc-to-nas', 'nas-to-pc'];
const name = ref(props.sync?.name ?? '');
const pcPath = ref(props.sync?.pcPath ?? '');
const nasPath = ref(props.sync?.nasPath ?? '');
const mode = ref<SyncMode>(props.sync?.mode ?? 'two-way');

const ready = computed(
  () =>
    name.value.trim().length > 0 &&
    pcPath.value.length > 0 &&
    (props.sync ? true : nasPath.value.includes('/')),
);

async function browse() {
  const folder = await pickFolder().catch(() => null);
  if (folder) {
    pcPath.value = folder;
    if (!name.value.trim()) {
      name.value = folder.split(/[\\/]/).findLast(Boolean) ?? '';
    }
  }
}

function save() {
  if (!ready.value || props.busy) {
    return;
  }
  if (props.sync) {
    emit('change', props.sync.id, {
      name: name.value.trim(),
      pcPath: pcPath.value,
      mode: mode.value,
    });
  } else {
    emit('create', {
      name: name.value.trim(),
      nasPath: nasPath.value,
      pcPath: pcPath.value,
      mode: mode.value,
    });
  }
}

</script>

<template>
  <CommonModal :title="sync ? t('editSyncTitle') : t('newSyncTitle')" @close="emit('close')">
    <form class="rule_form" @submit.prevent="save">
      <label class="field">
        {{ t('name') }}
        <input v-model="name" maxlength="80" data-testid="rule_name" />
      </label>

      <div class="field">
        {{ t('pcFolder') }}
        <div class="picker">
          <span class="picker_path" data-testid="rule_pc_path">{{ pcPath || t('noFolder') }}</span>
          <button type="button" class="secondary_button" data-testid="rule_browse" @click="browse">
            <span
              class="icon"
              :style="{ '--icon_url': `url(${folderIcon})` }"
              aria-hidden="true"
            ></span>
            {{ t('browse') }}
          </button>
        </div>
      </div>

      <div class="field">
        {{ t('nasFolder') }}
        <span v-if="sync" class="picker_path">{{ sync.nasLabel }}</span>
        <NasFolderBrowser v-else v-model="nasPath" />
      </div>

      <fieldset class="modes">
        <legend>{{ t('mode') }}</legend>
        <label v-for="item in MODES" :key="item" class="mode_option">
          <input v-model="mode" type="radio" name="mode" :value="item" />
          <span>
            <strong>{{ t(`mode_${item}`) }}</strong>
            <small class="hint">{{ t(`modeHint_${item}`) }}</small>
          </span>
        </label>
      </fieldset>

      <p v-if="error" class="error_message" role="alert">{{ errorText(error) }}</p>

      <div class="modal_actions">
        <button type="button" class="secondary_button" @click="emit('close')">
          {{ t('cancel') }}
        </button>
        <button
          type="submit"
          class="primary_button"
          :disabled="busy || !ready"
          data-testid="rule_save"
        >
          {{ t('save') }}
        </button>
      </div>
    </form>
  </CommonModal>
</template>

<style scoped lang="scss">

.rule_form {
  @extend %stack;
  gap: $space_4;
}

.picker {
  @extend %fx_base;
  gap: $space_2;
}

.picker_path {
  @extend %text_ellipsis;
  flex: 1;
  padding: $space_2 $space_3;
  border-radius: $radius_small;
  background: var(--surface-soft);
  font-weight: 400;
}

.modes {
  @extend %stack;
  gap: $space_2;
  border: 0;

  legend {
    font-size: $font_small;
    font-weight: 600;
    margin-bottom: $space_2;
  }
}

.mode_option {
  @extend %fx_row;
  gap: $space_3;
  align-items: flex-start;

  input {
    min-height: 0;
    margin-top: $space_1;
  }

  span {
    @extend %stack;
  }
}

.error_message {
  margin: 0;
}

</style>
