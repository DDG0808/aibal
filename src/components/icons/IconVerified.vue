<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
  >
    <path
      d="M22 11.08V12a10 10 0 11-5.93-9.14"
      :stroke="color"
      :stroke-width="strokeWidth"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <polyline
      points="22,4 12,14.01 9,11.01"
      :stroke="color"
      :stroke-width="strokeWidth"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  size?: number | string;
  strokeWidth?: number;
  color?: string;
}>(), {
  size: 16,
  strokeWidth: 2,
  color: 'var(--color-accent-blue)',
});
</script>
