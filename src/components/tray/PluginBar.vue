<script setup lang="ts">
/**
 * 底部管理按钮组件
 * 显示"管理插件"按钮
 */

const emit = defineEmits<{
  (e: 'manage'): void;
}>();

const handleManage = () => {
  emit('manage');
};
</script>

<template>
  <div class="plugin-bar">
    <!-- 管理按钮 -->
    <button
      class="manage-btn"
      @click="handleManage"
    >
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <rect
          x="3"
          y="3"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="14"
          y="3"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="3"
          y="14"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="14"
          y="14"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
      </svg>
      <span>管理插件</span>
    </button>
  </div>
</template>

<style scoped>
.plugin-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: var(--spacing-md);
  border-top: 1px solid var(--color-border);
}

.manage-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.manage-btn:hover {
  background: var(--color-bg);
  border-color: var(--color-text-secondary);
}

.manage-btn svg {
  color: var(--color-text-secondary);
}
</style>
