<script setup lang="ts">

import { ref } from 'vue';
import { useClient } from '@/composables/useClient';
import { useI18n } from '@/i18n/i18n';
import {
  createSync,
  disconnectNas,
  moveNas,
  removeSync,
  savePreferences,
  setPaused,
  updateSync,
} from '@/services/client';
import type { MainTab, NewSync, Preferences, SyncChange, SyncView } from '@/types';

import CommonSpinner from '@/components/common/CommonSpinner.vue';
import ConfirmModal from '@/components/common/ConfirmModal.vue';
import ConnectWizard from '@/components/ConnectWizard.vue';
import HistoryList from '@/components/HistoryList.vue';
import OptionsModal from '@/components/OptionsModal.vue';
import PageHeader from '@/components/layout/PageHeader.vue';
import StatusBadge from '@/components/StatusBadge.vue';
import SyncList from '@/components/SyncList.vue';
import SyncRuleModal from '@/components/SyncRuleModal.vue';
import TitleBar from '@/components/TitleBar.vue';

import folderIcon from '@/assets/icons/folder.svg?no-inline';
import historyIcon from '@/assets/icons/history.svg?no-inline';
import pauseIcon from '@/assets/icons/pause.svg?no-inline';
import playIcon from '@/assets/icons/play.svg?no-inline';
import plusIcon from '@/assets/icons/plus.svg?no-inline';
import settingsIcon from '@/assets/icons/settings.svg?no-inline';

const { t, errorText } = useI18n();
const { snapshot, history, error, busy, refresh, run } = useClient(200);

const editing = ref<SyncView | null | undefined>(undefined);
const removing = ref<SyncView | null>(null);
const optionsOpen = ref(false);

const TABS: MainTab[] = ['syncs', 'history'];
const tab = ref<MainTab>('syncs');

/** Moves the selection between the tabs with the arrow keys, like a native tab list. */
function moveTab(step: number) {
  const next = TABS[(TABS.indexOf(tab.value) + step + TABS.length) % TABS.length]!;
  tab.value = next;
  document.getElementById(`tab_${next}`)?.focus();
}

async function act(action: () => Promise<unknown>, close: () => void) {
  if (await run(action)) {
    close();
  }
}

const closeRule = () => (editing.value = undefined);
const create = (sync: NewSync) => act(() => createSync(sync), closeRule);
const change = (id: string, sync: SyncChange) => act(() => updateSync(id, sync), closeRule);
const pause = (sync: SyncView, paused: boolean) => run(() => updateSync(sync.id, { paused }));
const remove = () =>
  act(
    () => removeSync(removing.value?.id ?? ''),
    () => (removing.value = null),
  );
const toggleAll = () => run(() => setPaused(!snapshot.value?.paused));
const move = (host: string, port: number) =>
  act(
    () => moveNas(host, port),
    () => (optionsOpen.value = false),
  );
const disconnect = () => act(disconnectNas, () => (optionsOpen.value = false));
const save = (preferences: Preferences) =>
  act(
    () => savePreferences(preferences),
    () => (optionsOpen.value = false),
  );

function openRule(sync: SyncView | null) {
  error.value = null;
  editing.value = sync;
}

</script>

<template>
  <div class="app">
    <TitleBar />

    <main class="view" data-testid="main_view">
      <div class="page_intro">
        <div class="intro_copy">
          <h1>{{ t('appTitle') }}</h1>
          <p class="body_copy">{{ t('appDescription') }}</p>
        </div>
        <div class="intro_actions">
          <PageHeader />
          <button
            v-if="snapshot?.link"
            class="icon_button"
            :aria-label="t('options')"
            :title="t('options')"
            data-testid="open_options"
            @click="optionsOpen = true"
          >
            <span
              class="icon"
              :style="{ '--icon_url': `url(${settingsIcon})` }"
              aria-hidden="true"
            ></span>
          </button>
        </div>
      </div>

      <output v-if="!snapshot" class="empty_state loading_state">
        <CommonSpinner />
        {{ t('loading') }}
      </output>

      <ConnectWizard v-else-if="!snapshot.link" @connected="refresh" />

      <template v-else>
        <section class="status_panel" data-testid="status_panel">
          <StatusBadge :status="snapshot.status" />
          <span class="status_host">{{ snapshot.link.host }}</span>
          <output v-if="snapshot.problem" class="status_problem">
            {{ errorText(snapshot.problem) }}
          </output>
          <button
            class="secondary_button"
            :disabled="busy"
            data-testid="toggle_all"
            @click="toggleAll"
          >
            <span
              class="icon"
              :style="{ '--icon_url': `url(${snapshot.paused ? playIcon : pauseIcon})` }"
              aria-hidden="true"
            ></span>
            {{ snapshot.paused ? t('resumeAll') : t('pauseAll') }}
          </button>
        </section>

        <p v-if="error && editing === undefined && !optionsOpen" class="error_message" role="alert">
          {{ errorText(error) }}
        </p>

        <section class="panel" data-testid="sync_panel">
          <div class="panel_header">
            <div
              class="panel_tabs"
              role="tablist"
              :aria-label="t('appDescription')"
              @keydown.right.prevent="moveTab(1)"
              @keydown.left.prevent="moveTab(-1)"
            >
              <button
                id="tab_syncs"
                type="button"
                role="tab"
                class="panel_tab"
                :class="{ active: tab === 'syncs' }"
                :aria-selected="tab === 'syncs'"
                aria-controls="panel_syncs"
                :tabindex="tab === 'syncs' ? 0 : -1"
                data-testid="tab_syncs"
                @click="tab = 'syncs'"
              >
                <span
                  class="icon"
                  :style="{ '--icon_url': `url(${folderIcon})` }"
                  aria-hidden="true"
                ></span>
                {{ t('syncsTitle') }}
                <span class="count">{{ snapshot.syncs.length }}</span>
              </button>
              <button
                id="tab_history"
                type="button"
                role="tab"
                class="panel_tab"
                :class="{ active: tab === 'history' }"
                :aria-selected="tab === 'history'"
                aria-controls="panel_history"
                :tabindex="tab === 'history' ? 0 : -1"
                data-testid="tab_history"
                @click="tab = 'history'"
              >
                <span
                  class="icon"
                  :style="{ '--icon_url': `url(${historyIcon})` }"
                  aria-hidden="true"
                ></span>
                {{ t('historyTitle') }}
              </button>
            </div>
            <button
              v-if="tab === 'syncs'"
              class="primary_button"
              data-testid="new_sync"
              @click="openRule(null)"
            >
              <span
                class="icon"
                :style="{ '--icon_url': `url(${plusIcon})` }"
                aria-hidden="true"
              ></span>
              {{ t('newSync') }}
            </button>
          </div>

          <div
            v-if="tab === 'syncs'"
            id="panel_syncs"
            role="tabpanel"
            aria-labelledby="tab_syncs"
            data-testid="panel_syncs"
          >
            <p class="body_copy panel_description">{{ t('syncsDescription') }}</p>
            <SyncList
              v-if="snapshot.syncs.length"
              :syncs="snapshot.syncs"
              :busy="busy"
              @pause="pause"
              @edit="openRule"
              @remove="removing = $event"
            />
            <div v-else class="empty_state">
              <span
                class="icon empty_icon"
                :style="{ '--icon_url': `url(${folderIcon})` }"
                aria-hidden="true"
              ></span>
              <h3>{{ t('emptySyncsTitle') }}</h3>
              <p class="body_copy">{{ t('emptySyncsDescription') }}</p>
            </div>
          </div>
          <div
            v-else
            id="panel_history"
            role="tabpanel"
            aria-labelledby="tab_history"
            data-testid="panel_history"
          >
            <p class="body_copy panel_description">{{ t('historyDescription') }}</p>
            <HistoryList :entries="history" :syncs="snapshot.syncs" />
          </div>
        </section>

        <SyncRuleModal
          v-if="editing !== undefined"
          :sync="editing"
          :busy="busy"
          :error="error"
          @close="closeRule"
          @create="create"
          @change="change"
        />
        <ConfirmModal
          v-if="removing"
          :title="t('removeSyncTitle')"
          :description="t('removeSyncDescription')"
          :action="t('remove')"
          :busy="busy"
          @close="removing = null"
          @confirm="remove"
        />
        <OptionsModal
          v-if="optionsOpen"
          :link="snapshot.link"
          :preferences="snapshot.preferences"
          :busy="busy"
          :error="error"
          @close="optionsOpen = false"
          @move="move"
          @disconnect="disconnect"
          @save="save"
        />
      </template>
    </main>
  </div>
</template>

<style scoped lang="scss">

.app {
  @extend %stack;
  flex: 1;
  min-height: 0;
}

.view {
  @extend %stack;
  flex: 1;
  width: 100%;
  max-width: $view_width;
  margin-inline: auto;
  min-height: 0;
  overflow: auto;
  padding: $space_8;
  gap: $space_6;
}

.page_intro {
  @extend %fx_between_center;
  align-items: flex-start;
  gap: $space_6;
}

.intro_copy {
  max-width: $intro_width;
}

.intro_actions {
  @extend %fx_base;
  gap: $space_2;
}

h1 {
  font-size: $font_heading;
  font-weight: 650;
  margin-bottom: $space_2;
}

.loading_state {
  min-height: $empty_height;
  color: var(--muted);
}

.status_panel {
  @extend %fx_base;
  @include surface($radius: $radius_large, $shadow: var(--panel-shadow));
  flex-wrap: wrap;
  gap: $space_3;
  padding: $space_4 $space_6;
  flex-shrink: 0;

  button {
    margin-left: auto;
  }
}

.status_host {
  @extend %muted_label;
}

.status_problem {
  flex-basis: 100%;
  order: 3;
  color: var(--danger);
  font-size: $font_small;
}

.panel {
  @include surface($radius: $radius_large, $shadow: var(--panel-shadow));
  overflow: hidden;
  flex-shrink: 0;
}

.panel_header {
  @extend %panel_header;
  flex-wrap: wrap;
}

.panel_tabs {
  @extend %fx_base;
  gap: $space_1;
  padding: $space_1;
  border-radius: $radius_medium;
  background: var(--surface-soft);
}

.panel_tab {
  @extend %fx_base;
  gap: $space_2;
  padding: $space_2 $space_4;
  border: 1px solid transparent;
  border-radius: $radius_small;
  color: var(--muted);
  background: transparent;
  font: inherit;
  font-size: $font_body;
  font-weight: 600;
  cursor: pointer;

  &:hover {
    color: var(--theme-foreground);
  }

  &.active {
    border-color: var(--border);
    color: var(--theme-foreground);
    background: var(--surface);
  }

  .icon {
    color: var(--accent);
  }
}

.panel_description {
  padding: $space_4 $space_6;
  font-size: $font_small;
}

.count {
  @extend %muted_label;
  display: inline-grid;
  place-items: center;
  min-width: $count_size;
  height: $count_size;
  padding-inline: $space_2;
  border-radius: $radius_small;
  background: var(--surface-soft);
}

.empty_state {
  padding: $space_12 $space_6;

  h3 {
    font-size: $font_title;
    font-weight: 600;
  }
}

.empty_icon {
  @include square($empty_icon_size);
  color: var(--accent);
}

.error_message {
  margin: 0;
}

@include compact {
  .view {
    padding: $space_5;
  }

  .page_intro {
    flex-wrap: wrap;
  }
}

</style>
