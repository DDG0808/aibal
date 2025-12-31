<script setup lang="ts">
/**
 * Phase 8.2: 系统状态卡片
 * 显示整体系统状态和监控插件数量
 * 参考设计: System Status - Monitored by 3 plugins
 */
import { computed } from 'vue';
import type { HealthStatus } from '@/types';

interface Props {
  /** 系统状态 */
  status: HealthStatus;
  /** 活跃插件数量 */
  activePluginCount: number;
}

const props = withDefaults(defineProps<Props>(), {
  status: 'healthy',
  activePluginCount: 0,
});

const emit = defineEmits<{
  (e: 'click'): void;
}>();

const statusInfo = computed(() => {
  switch (props.status) {
    case 'healthy':
      return { color: '#22c55e', label: '系统状态' };
    case 'degraded':
      return { color: '#f97316', label: '系统警告' };
    case 'unhealthy':
      return { color: '#ef4444', label: '系统错误' };
    default:
      return { color: '#22c55e', label: '系统状态' };
  }
});

const handleClick = () => {
  emit('click');
};
</script>

<template>
  <div
    class="system-status-card"
    role="button"
    tabindex="0"
    @click="handleClick"
    @keydown.enter="handleClick"
    @keydown.space.prevent="handleClick"
  >
    <div class="status-left">
      <div
        class="status-indicator"
        :style="{ backgroundColor: statusInfo.color }"
      />
      <div class="status-info">
        <span class="status-title">{{ statusInfo.label }}</span>
        <span class="status-subtitle">
          由 {{ activePluginCount }} 个插件监控
        </span>
      </div>
    </div>
    <div class="status-arrow">
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
      >
        <path
          d="M9 18l6-6-6-6"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.system-status-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  background: #1e1e1e;
  border-radius: 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.system-status-card:hover {
  background: #252525;
}

.system-status-card:focus {
  outline: 2px solid #f97316;
  outline-offset: 2px;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-indicator {
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
  font-size: 0.95rem;
  font-weight: 600;
  color: #ffffff;
}

.status-subtitle {
  font-size: 0.8rem;
  color: #666;
}

.status-arrow {
  color: #555;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
