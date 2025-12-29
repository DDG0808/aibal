<script setup lang="ts">
/**
 * 状态卡片组件
 * 显示系统状态、描述和监控插件数量
 */
import { computed } from 'vue';
import type { StatusIndicator } from '@/types';

interface Props {
  /** 状态指示 */
  indicator: StatusIndicator;
  /** 状态描述 */
  description?: string;
  /** 监控插件数量 */
  monitoredCount?: number;
  /** 是否可展开 */
  expandable?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  description: 'System Status',
  monitoredCount: 0,
  expandable: false,
});

const emit = defineEmits<{
  (e: 'expand'): void;
}>();

// 状态显示配置
const statusConfig = computed(() => {
  switch (props.indicator) {
    case 'none':
      return { text: 'Operational', color: 'var(--color-accent-green)' };
    case 'minor':
      return { text: 'Minor Issues', color: 'var(--color-accent)' };
    case 'major':
      return { text: 'Major Outage', color: 'var(--color-accent-red)' };
    case 'critical':
      return { text: 'Critical', color: 'var(--color-accent-red)' };
    default:
      return { text: 'Unknown', color: 'var(--color-text-secondary)' };
  }
});

// 副标题显示
const subtitle = computed(() => {
  if (props.monitoredCount > 0) {
    return `Monitored by ${props.monitoredCount} plugin${props.monitoredCount > 1 ? 's' : ''}`;
  }
  // 避免与 description 重复，显示状态文本
  return statusConfig.value.text;
});

const handleExpand = () => {
  if (props.expandable) {
    emit('expand');
  }
};

// 键盘可达性支持
const handleKeydown = (e: KeyboardEvent) => {
  if (props.expandable && (e.key === 'Enter' || e.key === ' ')) {
    e.preventDefault();
    emit('expand');
  }
};
</script>

<template>
  <div
    class="status-card"
    :class="{ expandable }"
    :role="expandable ? 'button' : undefined"
    :tabindex="expandable ? 0 : undefined"
    :aria-label="expandable ? `${description} - ${statusConfig.text}` : undefined"
    @click="handleExpand"
    @keydown="handleKeydown"
  >
    <div class="status-left">
      <span
        class="status-dot"
        :style="{ backgroundColor: statusConfig.color }"
      ></span>
      <div class="status-info">
        <span class="status-title">{{ description }}</span>
        <span class="status-subtitle">{{ subtitle }}</span>
      </div>
    </div>

    <div v-if="expandable" class="status-right">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M9 18l6-6-6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
  </div>
</template>

<style scoped>
.status-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-md);
  border: 1px dashed var(--color-border);
}

.status-card.expandable {
  cursor: pointer;
  transition: background-color 0.2s ease;
}

.status-card.expandable:hover {
  background: var(--color-bg);
}

.status-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.status-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.status-subtitle {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.status-right {
  color: var(--color-text-secondary);
}
</style>
