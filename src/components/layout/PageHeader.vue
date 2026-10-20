<script setup lang="ts">

import { computed } from 'vue';
import { useI18n } from '@/i18n/i18n';
import { useTheme } from '@/composables/useTheme';
import { setAppearance } from '@/services/client';
import type { Locale, Theme } from '@/types';

import sunIcon from '@/assets/icons/sun.svg?no-inline';
import globeIcon from '@/assets/icons/globe.svg?no-inline';
import CommonSelect from '@/components/common/CommonSelect.vue';

const { t, locale, setLocale } = useI18n();
const { theme, setTheme } = useTheme();

function keep() {
  setAppearance({ locale: locale.value, theme: theme.value }).catch(() => undefined);
}

const selectedLocale = computed<Locale>({
  get: () => locale.value,
  set: (value) => {
    setLocale(value);
    keep();
  },
});

const selectedTheme = computed<Theme>({
  get: () => theme.value,
  set: (value) => {
    setTheme(value);
    keep();
  },
});

</script>

<template>
  <header class="page_header" data-testid="page_header">
    <CommonSelect
      id="theme"
      v-model="selectedTheme"
      :label="t('theme')"
      :icon="sunIcon"
      :options="[
        { value: 'dark', label: t('dark') },
        { value: 'light', label: t('light') },
      ]"
    />
    <CommonSelect
      id="language"
      v-model="selectedLocale"
      :label="t('language')"
      :icon="globeIcon"
      :options="[
        { value: 'it', label: t('italian') },
        { value: 'en', label: t('english') },
      ]"
    />
  </header>
</template>

<style scoped lang="scss">

.page_header {
  @extend %fx_base;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: $space_2;
  padding-top: $space_2;
  flex-shrink: 0;
}

@include compact {
  .page_header {
    padding: 0;
  }
}

</style>
