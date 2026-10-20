<script setup lang="ts">

import { useI18n } from '@/i18n/i18n';
import { fileName, formatMoment } from '@/services/format';
import type { HistoryEntry, SyncView } from '@/types';

import downloadIcon from '@/assets/icons/download.svg?no-inline';
import uploadIcon from '@/assets/icons/upload.svg?no-inline';
import warningIcon from '@/assets/icons/warning.svg?no-inline';

interface Props {
  entries: HistoryEntry[];
  syncs: SyncView[];
  compact?: boolean;
}

const props = defineProps<Props>();

const { t, locale } = useI18n();

const syncName = (id: string) => props.syncs.find((sync) => sync.id === id)?.name ?? id;

function iconOf(entry: HistoryEntry) {
  if (entry.action === 'failed' || entry.conflict) {
    return warningIcon;
  }

  return entry.direction === 'upload' ? uploadIcon : downloadIcon;
}

</script>

<template>
  <ul v-if="entries.length" class="history" :class="{ history_compact: compact }">
    <li
      v-for="(entry, index) in entries"
      :key="`${entry.time}-${entry.path}-${index}`"
      class="history_entry"
      :class="{ entry_problem: entry.action === 'failed' || entry.conflict }"
      data-testid="history_entry"
    >
      <span
        class="icon entry_icon"
        :style="{ '--icon_url': `url(${iconOf(entry)})` }"
        :title="entry.direction === 'upload' ? t('toNas') : t('toPc')"
        aria-hidden="true"
      ></span>
      <span class="entry_text">
        <strong :title="entry.path">{{ fileName(entry.path) }}</strong>
        <small>
          {{ syncName(entry.folder) }} · {{ t(`action_${entry.action}`) }}
          <template v-if="entry.conflict">· {{ t('conflict') }}</template>
        </small>
      </span>
      <time class="entry_time" :datetime="entry.time">
        {{ formatMoment(entry.time, locale, t('never')) }}
      </time>
    </li>
  </ul>

  <!-- Empty state -->
  <p v-else class="hint history_empty" data-testid="history_empty">{{ t('historyEmpty') }}</p>
</template>

<style scoped lang="scss">

.history_entry {
  @extend %fx_base;
  gap: $space_3;
  padding: $space_3 $space_4;
  border-top: $border_width solid var(--border);
}

.history_compact .history_entry {
  padding: $space_2 $space_3;
}

.entry_icon {
  color: var(--accent);
}

.entry_problem .entry_icon {
  color: var(--danger);
}

.entry_text {
  @extend %stack;
  flex: 1;
  min-width: 0;

  strong {
    @extend %text_ellipsis;
    font-weight: 500;
  }

  small {
    @extend %muted_label;
    @extend %text_ellipsis;
  }
}

.entry_time {
  @extend %muted_label;
  white-space: nowrap;
}

.history_empty {
  padding: $space_6;
  text-align: center;
}

</style>
