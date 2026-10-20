<script setup lang="ts">

import { onBeforeUnmount, onMounted, ref, useId } from 'vue';
import { useI18n } from '@/i18n/i18n';
import closeIcon from '@/assets/icons/close.svg?no-inline';

interface Props {
  title: string;
}

defineProps<Props>();

const { t } = useI18n();

const emit = defineEmits<{ close: [] }>();

const dialog = ref<HTMLDialogElement>();
const titleId = useId();

onMounted(() => {
  dialog.value?.showModal();
});

onBeforeUnmount(() => {
  dialog.value?.close();
});

</script>

<template>
  <Teleport to="body">
    <dialog
      ref="dialog"
      :aria-labelledby="titleId"
      @cancel.prevent="emit('close')"
      @click="$event.target === dialog && emit('close')"
    >
      <div class="modal_header">
        <h2 :id="titleId">{{ title }}</h2>

        <button class="icon_button" :aria-label="t('close')" @click="emit('close')">
          <span
            class="icon"
            :style="{ '--icon_url': `url(${closeIcon})` }"
            aria-hidden="true"
          ></span>
        </button>
      </div>

      <div class="modal_body"><slot></slot></div>
    </dialog>
  </Teleport>
</template>

<style scoped lang="scss">

dialog {
  @include surface($radius: $radius_large, $shadow: $shadow_popup);
  position: fixed;
  inset: 0;
  margin: auto;
  width: min($modal_width, calc(100vw - $space_8));
  max-height: calc(100dvh - $space_8);
  overflow: auto;
  padding: 0;
  color: var(--theme-foreground);

  &::backdrop {
    background: var(--modal-backdrop);
    backdrop-filter: blur(4px);
  }
}

.modal_header {
  @extend %panel_header;
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--surface);
}

h2 {
  font-size: $font_title;
  font-weight: 600;
}

button {
  color: var(--muted);
}

.modal_body {
  padding: $space_6;
}

@media (max-height: 540px), (max-width: 480px) {
  .modal_header,
  .modal_body {
    padding: $space_4;
  }
}

</style>
