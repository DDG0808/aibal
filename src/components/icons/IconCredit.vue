<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
  >
    <rect
      x="1"
      y="4"
      width="22"
      height="16"
      rx="2"
      stroke="currentColor"
      :stroke-width="strokeWidth"
    />
    <line
      x1="1"
      y1="10"
      x2="23"
      y2="10"
      stroke="currentColor"
      :stroke-width="strokeWidth"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  size?: number | string;
  strokeWidth?: number;
}>(), {
  size: 24,
  strokeWidth: 2,
});
</script>
