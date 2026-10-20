<script setup lang="ts">

import { computed } from 'vue';
import { useI18n } from '@/i18n/i18n';
import type { Status } from '@/types';

import checkIcon from '@/assets/icons/check.svg?no-inline';
import offlineIcon from '@/assets/icons/offline.svg?no-inline';
import pauseIcon from '@/assets/icons/pause.svg?no-inline';
import syncIcon from '@/assets/icons/sync.svg?no-inline';
import warningIcon from '@/assets/icons/warning.svg?no-inline';

interface Props {
  status: Status;
}

const props = defineProps<Props>();

const { t } = useI18n();

const ICONS: Record<Status, string> = {
  unconfigured: offlineIcon,
  paused: pauseIcon,
  disconnected: offlineIcon,
  syncing: syncIcon,
  synced: checkIcon,
  error: warningIcon,
};

const icon = computed(() => ICONS[props.status]);

</script>

<template>
  <span class="status_badge" :class="`status_${status}`" data-testid="status_badge">
    <span class="icon" :style="{ '--icon_url': `url(${icon})` }" aria-hidden="true"></span>
    {{ t(`status_${status}`) }}
  </span>
</template>

<style scoped lang="scss">

.status_badge {
  @extend %fx_base;
  gap: $space_2;
  padding: $space_1 $space_3;
  border-radius: $radius_small;
  font-size: $font_small;
  font-weight: 600;
  color: var(--muted);
  background: var(--surface-soft);
  white-space: nowrap;
}

.status_synced {
  color: var(--success);
  background: var(--success-soft);
}

.status_syncing {
  color: var(--accent);
  background: var(--accent-soft);

  .icon {
    animation: status_spin $duration_slow * 2 linear infinite;
  }
}

.status_disconnected,
.status_error {
  color: var(--danger);
  background: var(--danger-soft);
}

.status_paused {
  color: var(--warning);
  background: var(--warning-soft);
}

@keyframes status_spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .status_syncing .icon {
    animation: none;
  }
}

</style>
