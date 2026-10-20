<script setup lang="ts">

import { computed, onMounted, ref } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { errorCode, nasFolders } from '@/services/client';
import type { Choice, ClientError } from '@/types';
import CommonSpinner from '@/components/common/CommonSpinner.vue';
import folderIcon from '@/assets/icons/folder.svg?no-inline';
import chevronIcon from '@/assets/icons/chevron-right.svg?no-inline';

const selected = defineModel<string>({ required: true });

const { t, errorText } = useI18n();

const trail = ref<Choice[]>([]);
const folders = ref<Choice[]>([]);
const loading = ref(false);
const error = ref<ClientError | null>(null);

const label = computed(() => trail.value.map((step) => step.name).join(' / '));

async function open(path: string) {
  loading.value = true;
  error.value = null;
  try {
    folders.value = await nasFolders(path);
  } catch (reason) {
    folders.value = [];
    error.value = errorCode(reason);
  } finally {
    loading.value = false;
  }
}

async function enter(folder: Choice) {
  trail.value = [...trail.value, folder];
  selected.value = folder.path;
  await open(folder.path);
}

async function up() {
  trail.value = trail.value.slice(0, -1);
  const parent = trail.value.at(-1);
  selected.value = parent?.path ?? '';
  await open(parent?.path ?? '');
}

onMounted(() => open(''));

</script>

<template>
  <div class="browser" data-testid="nas_browser">
    <div class="browser_path">
      <button
        type="button"
        class="secondary_button"
        :disabled="!trail.length || loading"
        @click="up"
      >
        {{ t('parentFolder') }}
      </button>
      <span class="browser_label" data-testid="nas_selected">{{ label || t('nasRoot') }}</span>
    </div>
    <output v-if="loading" class="empty_state browser_state">
      <CommonSpinner />
    </output>
    <p v-else-if="error" class="error_message" role="alert">{{ errorText(error) }}</p>
    <ul v-else-if="folders.length" class="browser_list">
      <li v-for="folder in folders" :key="folder.path">
        <button type="button" data-testid="nas_folder" @click="enter(folder)">
          <span
            class="icon"
            :style="{ '--icon_url': `url(${folderIcon})` }"
            aria-hidden="true"
          ></span>
          <span class="folder_name">{{ folder.name }}</span>
          <span
            class="icon"
            :style="{ '--icon_url': `url(${chevronIcon})` }"
            aria-hidden="true"
          ></span>
        </button>
      </li>
    </ul>
    <p v-else class="hint browser_state">{{ t('nasEmpty') }}</p>
  </div>
</template>

<style scoped lang="scss">

.browser {
  @include surface($background: var(--surface-soft));
  overflow: hidden;
}

.browser_path {
  @extend %fx_base;
  gap: $space_3;
  padding: $space_2 $space_3;
  border-bottom: $border_width solid var(--border);
}

.browser_label {
  @extend %text_ellipsis;
  font-weight: 600;
  font-size: $font_control;
}

.browser_list {
  max-height: $browser_height;
  overflow: auto;

  button {
    @extend %fx_base;
    gap: $space_3;
    width: 100%;
    padding: $space_2 $space_3;
    text-align: left;
  }
}

.folder_name {
  @extend %text_ellipsis;
  flex: 1;
}

.browser_state {
  padding: $space_4;
}

.error_message {
  margin: $space_3;
}

</style>
