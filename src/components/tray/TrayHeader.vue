<script setup lang="ts">
/**
 * 托盘弹窗头部组件
 * 显示应用标题、运行状态、刷新和设置按钮
 */
import { computed } from 'vue';

interface Props {
  /** 应用版本 */
  version?: string;
  /** 是否正在刷新 */
  isRefreshing?: boolean;
  /** 系统状态 (healthy/degraded/unhealthy) */
  systemStatus?: 'healthy' | 'degraded' | 'unhealthy';
}

const props = withDefaults(defineProps<Props>(), {
  version: '2.2',
  isRefreshing: false,
  systemStatus: 'healthy',
});

const emit = defineEmits<{
  (e: 'refresh'): void;
  (e: 'settings'): void;
}>();

// 状态指示器颜色
const statusColor = computed(() => {
  switch (props.systemStatus) {
    case 'healthy': return 'var(--color-accent-green)';
    case 'degraded': return 'var(--color-accent)';
    case 'unhealthy': return 'var(--color-accent-red)';
    default: return 'var(--color-accent-green)';
  }
});

const handleRefresh = () => {
  if (!props.isRefreshing) {
    emit('refresh');
  }
};

const handleSettings = () => {
  emit('settings');
};
</script>

<template>
  <header class="tray-header">
    <div class="header-left">
      <!-- Logo 图标 -->
      <div class="logo-icon">
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <rect
            x="3"
            y="3"
            width="18"
            height="18"
            rx="4"
            stroke="currentColor"
            stroke-width="2"
          />
          <circle
            cx="8"
            cy="8"
            r="1.5"
            fill="currentColor"
          />
          <circle
            cx="12"
            cy="8"
            r="1.5"
            fill="currentColor"
          />
          <circle
            cx="16"
            cy="8"
            r="1.5"
            fill="currentColor"
          />
          <path
            d="M6 14h12M6 17h8"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </div>
      <div class="header-info">
        <h1 class="app-title">
          AI Tracker
        </h1>
        <div class="app-subtitle">
          <span
            class="status-dot"
            :style="{ backgroundColor: statusColor }"
          />
          <span>Core Runtime v{{ version }}</span>
        </div>
      </div>
    </div>

    <div class="header-actions">
      <!-- 刷新按钮 -->
      <button
        type="button"
        class="icon-btn"
        :class="{ 'is-refreshing': isRefreshing }"
        :disabled="isRefreshing"
        :aria-label="isRefreshing ? '正在刷新' : '刷新数据'"
        @click="handleRefresh"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M21 12a9 9 0 11-2.636-6.364"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
          <path
            d="M21 3v6h-6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>

      <!-- 设置按钮 -->
      <button
        type="button"
        class="icon-btn"
        aria-label="打开设置"
        @click="handleSettings"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <circle
            cx="12"
            cy="12"
            r="3"
            stroke="currentColor"
            stroke-width="2"
          />
          <path
            d="M12 1v3m0 16v3M4.22 4.22l2.12 2.12m11.32 11.32l2.12 2.12M1 12h3m16 0h3M4.22 19.78l2.12-2.12m11.32-11.32l2.12-2.12"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.tray-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-md);
  background: var(--color-bg);
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.logo-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
}

.header-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.app-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
  line-height: 1.2;
}

.app-subtitle {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.icon-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.icon-btn:hover {
  background: var(--color-bg-secondary);
  color: var(--color-text);
}

.icon-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.icon-btn.is-refreshing svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
