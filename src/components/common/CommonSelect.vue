<script setup lang="ts" generic="T extends string">

import { computed, nextTick, onBeforeUnmount, onMounted, ref, useId } from 'vue';
import type { SelectOption } from '@/types';
import checkIcon from '@/assets/icons/check.svg?no-inline';
import chevronIcon from '@/assets/icons/down.svg?no-inline';

interface Props {
  id: string;
  label: string;
  icon: string;
  options: SelectOption<T>[];
}

const props = defineProps<Props>();
const model = defineModel<T>({ required: true });
const root = ref<HTMLElement>();
const trigger = ref<HTMLButtonElement>();
const menu = ref<HTMLUListElement>();
const open = ref(false);
const menuId = useId();
const selectedIndex = computed(() =>
  Math.max(
    0,
    props.options.findIndex((option) => option.value === model.value),
  ),
);
const selectedLabel = computed(() => props.options[selectedIndex.value]?.label ?? '');
const navigationKeys = new Set(['ArrowDown', 'ArrowUp', 'Home', 'End']);

async function show(index = selectedIndex.value) {
  open.value = true;
  await nextTick();
  menu.value?.querySelectorAll('button')[index]?.focus();
}

function close(restoreFocus = false) {
  open.value = false;
  if (restoreFocus) {
    trigger.value?.focus();
  }
}

function choose(value: T) {
  model.value = value;
  close(true);
}

function navigate(event: KeyboardEvent) {
  if (event.key === 'Escape' && open.value) {
    event.preventDefault();
    event.stopPropagation();
    close(true);
    return;
  }
  if (!navigationKeys.has(event.key)) {
    return;
  }

  event.preventDefault();
  const buttons = Array.from(menu.value?.querySelectorAll('button') ?? []);
  const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
  let index = selectedIndex.value;
  if (event.key === 'Home') {
    index = 0;
  } else if (event.key === 'End') {
    index = props.options.length - 1;
  } else if (open.value && current >= 0) {
    const delta = event.key === 'ArrowDown' ? 1 : -1;
    index = (current + delta + props.options.length) % props.options.length;
  }

  show(index);
}

function outside(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) {
    close();
  }
}

function leave(event: FocusEvent) {
  if (!root.value?.contains(event.relatedTarget as Node | null)) {
    close();
  }
}

onMounted(() => document.addEventListener('pointerdown', outside));

onBeforeUnmount(() => document.removeEventListener('pointerdown', outside));

</script>

<template>
  <div ref="root" class="select_control" @keydown="navigate" @focusout="leave">
    <button
      :id="id"
      ref="trigger"
      type="button"
      class="select_trigger"
      :aria-label="`${label}: ${selectedLabel}`"
      :title="label"
      aria-haspopup="menu"
      :aria-expanded="open"
      :aria-controls="open ? menuId : undefined"
      @click="open ? close() : show()"
    >
      <span
        class="icon select_icon"
        :style="{ '--icon_url': `url(${icon})` }"
        aria-hidden="true"
      ></span>
      <span>{{ selectedLabel }}</span>
      <span
        class="icon select_chevron"
        :style="{ '--icon_url': `url(${chevronIcon})` }"
        aria-hidden="true"
      ></span>
    </button>
    <ul v-if="open" :id="menuId" ref="menu" class="select_menu" role="menu" :aria-label="label">
      <li v-for="option in options" :key="option.value" role="presentation">
        <button
          type="button"
          role="menuitemradio"
          :aria-checked="option.value === model"
          tabindex="-1"
          @click="choose(option.value)"
        >
          <span>{{ option.label }}</span>
          <span
            v-if="option.value === model"
            class="icon selected_check"
            :style="{ '--icon_url': `url(${checkIcon})` }"
            aria-hidden="true"
          ></span>
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped lang="scss">

.select_control {
  position: relative;
}

.select_trigger {
  @extend %fx_base;
  @include surface($radius: $radius_medium);
  gap: $space_2;
  min-width: 118px;
  min-height: 38px;
  padding: $space_2 $space_3;
  font-size: $font_control;
  font-weight: 500;
  color: var(--theme-foreground);

  &:hover,
  &[aria-expanded='true'] {
    background: var(--surface-soft);
    border-color: var(--accent-border);
  }
}

.select_icon {
  @include square(18px);
  color: var(--muted);
}

.select_chevron {
  @include square(14px);
  color: var(--muted);
  margin-left: auto;
}

.select_control:focus-within .select_icon {
  color: var(--accent);
}

.select_menu {
  @include surface($radius: $radius_medium, $shadow: $shadow_popup);
  position: absolute;
  top: calc(100% + #{$space_1});
  right: 0;
  z-index: 20;
  min-width: 100%;
  padding: $space_1;
  color: var(--theme-foreground);

  button {
    @extend %fx_between_center;
    gap: $space_4;
    width: 100%;
    min-height: $control_height;
    padding: $space_2;
    border-radius: $radius_small;
    font-size: $font_control;
    text-align: left;
    white-space: nowrap;

    &[aria-checked='true'] {
      background: var(--accent-soft);
      color: var(--accent);
      font-weight: 600;
    }

    &:focus-visible {
      outline-offset: -$focus_width;
    }
  }
}

.selected_check {
  @include square(16px);
}

</style>
