<script setup lang="ts">
/**
 * 系统状态卡片组件
 * 显示系统状态和启动插件数量
 */
import { computed } from 'vue';

interface Props {
  /** 启动插件数量 */
  runningCount?: number;
}

const props = withDefaults(defineProps<Props>(), {
  runningCount: 0,
});

// 状态配置
const statusConfig = computed(() => {
  if (props.runningCount === 0) {
    return { color: 'var(--color-text-tertiary)' };
  }
  return { color: 'var(--color-accent-green)' };
});

// 描述文字
const description = computed(() => {
  if (props.runningCount === 0) {
    return '暂无插件';
  }
  return `共 ${props.runningCount} 个启动插件`;
});
</script>

<template>
  <div class="status-card">
    <div class="status-left">
      <span
        class="status-dot"
        :style="{ backgroundColor: statusConfig.color }"
      />
      <div class="status-info">
        <span class="status-title">系统状态</span>
        <span class="status-subtitle">{{ description }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.status-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-sm) 0;
  border-top: 1px solid var(--color-border);
  margin-top: var(--spacing-sm);
}

.status-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.status-title {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.status-subtitle {
  font-size: 0.6875rem;
  color: var(--color-text-tertiary);
}
</style>
