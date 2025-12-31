<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
  >
    <circle
      cx="12"
      cy="12"
      r="10"
      stroke="currentColor"
      :stroke-width="strokeWidth"
    />
    <polyline
      points="12,6 12,12 16,14"
      stroke="currentColor"
      :stroke-width="strokeWidth"
      stroke-linecap="round"
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
