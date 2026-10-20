<script setup lang="ts">

import { useI18n } from '@/i18n/i18n';
import { formatMoment } from '@/services/format';
import type { SyncView } from '@/types';

import editIcon from '@/assets/icons/edit.svg?no-inline';
import deleteIcon from '@/assets/icons/delete.svg?no-inline';
import pauseIcon from '@/assets/icons/pause.svg?no-inline';
import playIcon from '@/assets/icons/play.svg?no-inline';
import folderIcon from '@/assets/icons/folder.svg?no-inline';

interface Props {
  syncs: SyncView[];
  busy: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  pause: [sync: SyncView, paused: boolean];
  edit: [sync: SyncView];
  remove: [sync: SyncView];
}>();

const { t, locale } = useI18n();

</script>

<template>
  <div class="table_scroll">
    <table>
      <thead>
        <tr>
          <th scope="col">{{ t('name') }}</th>
          <th scope="col">{{ t('pcFolder') }}</th>
          <th scope="col">{{ t('nasFolder') }}</th>
          <th scope="col">{{ t('state') }}</th>
          <th scope="col">{{ t('lastSync') }}</th>
          <th scope="col">{{ t('actions') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="sync in syncs" :key="sync.id" data-testid="sync_row">
          <td>
            <div class="sync_name">
              <span class="sync_symbol">
                <span
                  class="icon"
                  :style="{ '--icon_url': `url(${folderIcon})` }"
                  aria-hidden="true"
                ></span>
              </span>
              <span class="sync_text">
                <strong>{{ sync.name }}</strong>
                <small>{{ t(`mode_${sync.mode}`) }}</small>
              </span>
            </div>
          </td>
          <td class="path" :title="sync.pcPath">{{ sync.pcPath }}</td>
          <td class="path" :title="sync.nasLabel">{{ sync.nasLabel }}</td>
          <td>
            <span class="state_badge" :class="`state_${sync.state}`">
              {{ t(`state_${sync.state}`) }}
              <template v-if="sync.state === 'syncing' && sync.progress !== null">
                {{ Math.floor(sync.progress) }}%
              </template>
            </span>
          </td>
          <td class="moment">{{ formatMoment(sync.lastSync, locale, t('never')) }}</td>
          <td>
            <div class="row_actions">
              <button
                class="icon_button"
                :disabled="busy || sync.state === 'revoked' || sync.state === 'unchosen'"
                :aria-label="`${sync.state === 'paused' ? t('resume') : t('pause')} ${sync.name}`"
                :title="sync.state === 'paused' ? t('resume') : t('pause')"
                @click="emit('pause', sync, sync.state !== 'paused')"
              >
                <span
                  class="icon"
                  :style="{
                    '--icon_url': `url(${sync.state === 'paused' ? playIcon : pauseIcon})`,
                  }"
                  aria-hidden="true"
                ></span>
              </button>
              <button
                class="icon_button"
                :disabled="busy || sync.state === 'revoked'"
                :aria-label="`${t('edit')} ${sync.name}`"
                :title="t('edit')"
                @click="emit('edit', sync)"
              >
                <span
                  class="icon"
                  :style="{ '--icon_url': `url(${editIcon})` }"
                  aria-hidden="true"
                ></span>
              </button>
              <button
                class="icon_button"
                :disabled="busy"
                :aria-label="`${t('remove')} ${sync.name}`"
                :title="t('remove')"
                @click="emit('remove', sync)"
              >
                <span
                  class="icon"
                  :style="{ '--icon_url': `url(${deleteIcon})` }"
                  aria-hidden="true"
                ></span>
              </button>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped lang="scss">

.table_scroll {
  overflow: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;

  th:first-child,
  td:first-child {
    padding-left: $space_4;
  }

  th {
    @extend %muted_label;
    padding: $space_3;
    font-weight: 500;
    background: var(--surface-soft);
    white-space: nowrap;
  }

  td {
    padding: $space_3;
    border-top: $border_width solid var(--border);
    vertical-align: middle;
  }
}

.sync_name {
  @extend %fx_base;
  gap: $space_3;
  white-space: nowrap;
}

.sync_text {
  @extend %stack;

  small {
    @extend %muted_label;
  }
}

.sync_symbol {
  @extend %selected_control;
  display: inline-grid;
  place-items: center;
  @include square($control_size);
  flex-shrink: 0;
  border-radius: $radius_small;
}

.path {
  @extend %text_ellipsis;
  max-width: $path_width;
  color: var(--muted);
}

.moment {
  @extend %muted_label;
  white-space: nowrap;
}

.state_badge {
  @extend %status_dot;
}

.state_synced {
  color: var(--success);
  background: var(--success-soft);
}

.state_syncing {
  color: var(--accent);
  background: var(--accent-soft);
}

.state_paused,
.state_unchosen {
  color: var(--warning);
  background: var(--warning-soft);
}

.state_offline,
.state_error,
.state_revoked {
  color: var(--danger);
  background: var(--danger-soft);
}

.row_actions {
  @extend %fx_base;
  gap: $space_1;
}

</style>
