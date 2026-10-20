<script setup lang="ts">

interface Props {
  label: string;
  hint?: string;
  testid?: string;
}

const props = withDefaults(defineProps<Props>(), {
  hint: undefined,
  testid: undefined,
});

const checked = defineModel<boolean>({ default: false });

</script>

<template>
  <label class="common_switch">
    <span class="common_switch_text">
      <span class="common_switch_label">{{ props.label }}</span>
      <small v-if="props.hint" class="common_switch_hint">{{ props.hint }}</small>
    </span>
    <input
      v-model="checked"
      type="checkbox"
      role="switch"
      :aria-checked="checked"
      :name="props.testid"
      :data-testid="props.testid"
    />
    <span class="common_switch_track" aria-hidden="true"></span>
  </label>
</template>

<style scoped lang="scss">

.common_switch {
  @extend %fx_between_center;
  gap: $space_4;
  cursor: pointer;

  input {
    @extend %visually_hidden;

    &:checked + .common_switch_track {
      background: var(--accent-strong);

      &::after {
        transform: translate(calc(#{$space_8} - #{$space_5}), -50%);
      }
    }

    &:focus-visible + .common_switch_track {
      @include focus-ring;
    }
  }

  &_text {
    @extend %stack;
    min-width: 0;
  }

  &_label {
    font-size: $font_body;
    font-weight: 600;
  }

  &_hint {
    @extend %hint;
  }

  &_track {
    position: relative;
    flex-shrink: 0;
    width: $space_8;
    height: $space_5;
    border-radius: $radius_large;
    background: var(--border);
    transition: background $duration_fast ease;

    &::after {
      content: '';
      position: absolute;
      top: 50%;
      left: $border_width * 2;
      width: calc(#{$space_5} - #{$space_1});
      height: calc(#{$space_5} - #{$space_1});
      border-radius: $radius_round;
      background: var(--on-accent);
      transform: translateY(-50%);
      transition: transform $duration_fast ease;
    }
  }
}

@media (prefers-reduced-motion: reduce) {
  .common_switch_track,
  .common_switch_track::after {
    transition: none;
  }
}

</style>
