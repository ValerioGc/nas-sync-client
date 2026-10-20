<script setup lang="ts">

import { useI18n } from '@/i18n/i18n';

const { t } = useI18n();

</script>

<template>
  <div class="warning_message">
    <div>
      <strong>{{ t('setupRequired') }}</strong>
      <p>{{ t('setupRequiredDescription') }}</p>
    </div>
  </div>
</template>

<style scoped lang="scss">

strong {
  display: block;
  font-weight: 600;
}

p {
  margin-top: $space_1;
  color: var(--theme-foreground);
}

</style>
