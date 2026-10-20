<script setup lang="ts">

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@/i18n/i18n';

import closeIcon from '@/assets/icons/close.svg?no-inline';
import maximizeIcon from '@/assets/icons/maximize.svg?no-inline';
import minimizeIcon from '@/assets/icons/minimize.svg?no-inline';
import logo from '@/assets/logo.svg';

const { t } = useI18n();
const failed = ref(false);

async function windowCommand(command: string) {
  try {
    await invoke(command);
    failed.value = false;
  } catch {
    failed.value = true;
  }
}

</script>

<template>
  <header class="titlebar" data-testid="titlebar">
    <div
      class="titlebar_identity"
      @mousedown.left="windowCommand('start_window_dragging')"
      @dblclick="windowCommand('toggle_maximize_window')"
    >
      <img :src="logo" alt="" />
      <span>{{ t('appTitle') }}</span>
    </div>

    <div class="window_controls">
      <button
        class="icon_button"
        :aria-label="t('windowMinimize')"
        @click="windowCommand('minimize_window')"
      >
        <span
          class="icon"
          :style="{ '--icon_url': `url(${minimizeIcon})` }"
          aria-hidden="true"
        ></span>
      </button>
      <button
        class="icon_button"
        :aria-label="t('windowMaximize')"
        @click="windowCommand('toggle_maximize_window')"
      >
        <span
          class="icon"
          :style="{ '--icon_url': `url(${maximizeIcon})` }"
          aria-hidden="true"
        ></span>
      </button>
      <button
        class="icon_button close_button"
        :aria-label="t('close')"
        @click="windowCommand('close_window')"
      >
        <span class="icon" :style="{ '--icon_url': `url(${closeIcon})` }" aria-hidden="true"></span>
      </button>
    </div>

    <p v-if="failed" class="window_error" role="alert">{{ t('windowError') }}</p>
  </header>
</template>

<style scoped lang="scss">

.titlebar {
  @extend %fx_base;
  position: relative;
  height: $titlebar_height;
  flex-shrink: 0;
  background: var(--theme-background);
  user-select: none;

  &_identity {
    @extend %fx_base;
    flex: 1;
    gap: $space_3;
    height: 100%;
    padding: 0 $space_6;
    font-weight: 600;
  }
}

img {
  @include square($icon_size);
}

.window_controls {
  @extend %fx_row;
  gap: $space_1;
  padding-right: $space_3;
}

.close_button:hover {
  background: var(--close-hover);
  color: var(--on-close);
}

.window_error {
  position: absolute;
  right: $space_4;
  top: 100%;
  z-index: 5;
  padding: $space_3;
  background: var(--surface);
  color: var(--danger);
}

</style>
