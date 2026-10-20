<script setup lang="ts">

import CommonModal from '@/components/common/CommonModal.vue';
import { useI18n } from '@/i18n/i18n';

interface Props {
  title: string;
  description: string;
  action: string;
  busy: boolean;
}

defineProps<Props>();

const emit = defineEmits<{ close: []; confirm: [] }>();

const { t } = useI18n();

</script>

<template>
  <CommonModal :title="title" @close="emit('close')">
    <p class="body_copy">{{ description }}</p>
    <div class="modal_actions">
      <button class="secondary_button" @click="emit('close')">{{ t('cancel') }}</button>
      <button
        class="primary_button danger_button"
        :disabled="busy"
        data-testid="confirm_action"
        @click="emit('confirm')"
      >
        {{ action }}
      </button>
    </div>
  </CommonModal>
</template>
