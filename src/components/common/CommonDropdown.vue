<script setup lang="ts">

import { onBeforeUnmount, onMounted, ref, useId } from 'vue';
import type { DropdownItem } from '@/types';
import moreIcon from '@/assets/icons/more.svg?no-inline';

interface Props {
  label: string;
  items: DropdownItem[];
}

defineProps<Props>();

const emit = defineEmits<{ select: [id: string] }>();

const open = ref(false);
const root = ref<HTMLElement>();
const menuId = useId();

function choose(item: DropdownItem) {
  if (!item.disabled) {
    open.value = false;
    emit('select', item.id);
  }
}

function outside(event: MouseEvent) {
  if (open.value && !root.value?.contains(event.target as Node)) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener('mousedown', outside));

onBeforeUnmount(() => document.removeEventListener('mousedown', outside));

</script>

<template>
  <div ref="root" class="dropdown" @keydown.esc="open = false">
    <button
      class="icon_button"
      :aria-label="label"
      :title="label"
      aria-haspopup="menu"
      :aria-expanded="open"
      :aria-controls="menuId"
      data-testid="dropdown_toggle"
      @click="open = !open"
    >
      <span class="icon" :style="{ '--icon_url': `url(${moreIcon})` }" aria-hidden="true"></span>
    </button>
    <ul v-if="open" :id="menuId" class="dropdown_menu" role="menu">
      <li v-for="item in items" :key="item.id" role="presentation">
        <button
          role="menuitem"
          :disabled="item.disabled"
          :data-testid="`dropdown_${item.id}`"
          @click="choose(item)"
        >
          <span
            class="icon"
            :style="{ '--icon_url': `url(${item.icon})` }"
            aria-hidden="true"
          ></span>
          {{ item.label }}
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped lang="scss">

.dropdown {
  position: relative;
}

.dropdown_menu {
  @include surface($radius: $radius_medium, $shadow: $shadow_popup);
  position: absolute;
  top: calc(100% + #{$space_1});
  right: 0;
  z-index: 10;
  min-width: 220px;
  padding: $space_1;

  button {
    @extend %fx_base;
    gap: $space_3;
    width: 100%;
    padding: $space_2 $space_3;
    text-align: left;
    font-size: $font_control;
  }
}

</style>
