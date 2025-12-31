<script setup lang="ts">
/**
 * 底部栏组件
 * 显示系统状态 + 插件图标 + 管理插件按钮
 */
import { computed } from 'vue';

interface SimplePlugin {
  id: string;
  name: string;
  enabled?: boolean;
}

interface Props {
  /** 插件列表 */
  plugins?: SimplePlugin[];
  /** 启动插件数量 */
  runningCount?: number;
}

const props = withDefaults(defineProps<Props>(), {
  plugins: () => [],
  runningCount: 0,
});

const emit = defineEmits<{
  (e: 'manage'): void;
}>();

// 系统状态描述
const statusDescription = computed(() => {
  if (props.runningCount === 0) {
    return '暂无插件';
  }
  return `共 ${props.runningCount} 个启动插件`;
});

// 状态颜色
const statusColor = computed(() => {
  if (props.runningCount === 0) {
    return 'var(--color-text-tertiary)';
  }
  return 'var(--color-accent-green)';
});

const handleManage = () => {
  emit('manage');
};
</script>

<template>
  <div class="plugin-bar">
    <!-- 左侧：系统状态 -->
    <div class="status-section">
      <span
        class="status-dot"
        :style="{ backgroundColor: statusColor }"
      />
      <div class="status-info">
        <span class="status-title">系统状态</span>
        <span class="status-desc">{{ statusDescription }}</span>
      </div>
    </div>

    <!-- 右侧：管理按钮 -->
    <button
      class="manage-btn"
      @click="handleManage"
    >
      管理插件
    </button>
  </div>
</template>

<style scoped>
.plugin-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-secondary);
  border-top: 1px solid var(--color-border);
}

.status-section {
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
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.status-desc {
  font-size: 0.625rem;
  color: var(--color-text-tertiary);
}

.manage-btn {
  padding: var(--spacing-xs) var(--spacing-sm);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  font-size: 0.6875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.manage-btn:hover {
  background: var(--color-bg);
  color: var(--color-text);
  border-color: var(--color-text-secondary);
}
</style>
