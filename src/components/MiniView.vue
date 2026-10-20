<script setup lang="ts">

import { computed } from 'vue';
import { useClient } from '@/composables/useClient';
import { useI18n } from '@/i18n/i18n';
import { hideMini, openMain, quitApp, setPaused } from '@/services/client';
import type { DropdownItem } from '@/types';

import CommonDropdown from '@/components/common/CommonDropdown.vue';
import HistoryList from '@/components/HistoryList.vue';
import StatusBadge from '@/components/StatusBadge.vue';

import logo from '@/assets/logo.svg';
import openIcon from '@/assets/icons/open.svg?no-inline';
import pauseIcon from '@/assets/icons/pause.svg?no-inline';
import playIcon from '@/assets/icons/play.svg?no-inline';
import powerIcon from '@/assets/icons/power.svg?no-inline';

const RECENT = 8;

const { t, errorText } = useI18n();
const { snapshot, history, error, run } = useClient(RECENT);

const items = computed<DropdownItem[]>(() => [
  snapshot.value?.paused
    ? { id: 'resume', label: t('resumeAll'), icon: playIcon, disabled: !snapshot.value.link }
    : { id: 'pause', label: t('pauseAll'), icon: pauseIcon, disabled: !snapshot.value?.link },
  { id: 'open', label: t('openApp'), icon: openIcon },
  { id: 'quit', label: t('quit'), icon: powerIcon },
]);

async function open() {
  await openMain().catch(() => undefined);
}

async function select(id: string) {
  if (id === 'pause' || id === 'resume') {
    await run(() => setPaused(id === 'pause'));
  } else if (id === 'open') {
    await open();
  } else if (id === 'quit') {
    await quitApp().catch(() => undefined);
  }
}

</script>

<template>
  <main class="mini" data-testid="mini_view" @keydown.esc="hideMini().catch(() => undefined)">
    <header class="mini_header">
      <img :src="logo" alt="" />
      <div class="mini_identity">
        <strong>{{ t('appTitle') }}</strong>
        <small v-if="snapshot?.link">{{ snapshot.link.host }}</small>
      </div>
      <CommonDropdown :label="t('menu')" :items="items" @select="select" />
    </header>

    <div class="mini_status">
      <StatusBadge v-if="snapshot" :status="snapshot.status" />
      <p v-if="snapshot?.problem" class="hint">{{ errorText(snapshot.problem) }}</p>
      <p v-if="error" class="error_message" role="alert">{{ errorText(error) }}</p>
    </div>

    <section class="mini_recent" aria-labelledby="recent_title">
      <h2 id="recent_title">{{ t('recentFiles') }}</h2>
      <HistoryList :entries="history" :syncs="snapshot?.syncs ?? []" compact />
    </section>

    <footer class="mini_footer">
      <button class="secondary_button" data-testid="mini_open" @click="open">
        <span class="icon" :style="{ '--icon_url': `url(${openIcon})` }" aria-hidden="true"></span>
        {{ t('openApp') }}
      </button>
    </footer>
  </main>
</template>

<style scoped lang="scss">

.mini {
  @extend %stack;
  height: 100%;
  border: $border_width solid var(--border);
  background: var(--surface);
}

.mini_header {
  @extend %fx_base;
  gap: $space_3;
  padding: $space_3 $space_4;
  border-bottom: $border_width solid var(--border);

  img {
    @include square(28px);
  }
}

.mini_identity {
  @extend %stack;
  flex: 1;
  min-width: 0;

  small {
    @extend %muted_label;
    @extend %text_ellipsis;
  }
}

.mini_status {
  @extend %stack;
  gap: $space_2;
  padding: $space_3 $space_4;
  align-items: flex-start;

  .error_message {
    margin: 0;
  }
}

.mini_recent {
  flex: 1;
  min-height: 0;
  overflow: auto;

  h2 {
    @extend %muted_label;
    padding: $space_2 $space_4;
    font-weight: 600;
  }
}

.mini_footer {
  @extend %fx_row;
  justify-content: flex-end;
  padding: $space_3 $space_4;
  border-top: $border_width solid var(--border);
}

</style>
