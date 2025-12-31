<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
  >
    <path
      d="M22 12h-4l-3 9L9 3l-3 9H2"
      stroke="currentColor"
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
}>(), {
  size: 24,
  strokeWidth: 2,
});
</script>
