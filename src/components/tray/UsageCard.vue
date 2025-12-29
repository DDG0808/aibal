<script setup lang="ts">
/**
 * 使用量卡片组件
 * 显示百分比、进度条、重置时间和流量状态
 */
import { computed } from 'vue';
import type { UsageData } from '@/types';

interface Props {
  /** 使用量数据 */
  data: UsageData;
  /** 插件名称 */
  pluginName?: string;
  /** 插件 ID (用于显示标签) */
  pluginId?: string;
}

const props = withDefaults(defineProps<Props>(), {
  pluginName: 'Usage',
  pluginId: '',
});

// 安全的百分比值 (clamp 到 0-100)
const safePercentage = computed(() => {
  const p = props.data.percentage;
  if (typeof p !== 'number' || isNaN(p)) return 0;
  return Math.max(0, Math.min(100, p));
});

// 计算进度条颜色
const progressColor = computed(() => {
  const p = safePercentage.value;
  if (p >= 90) return 'var(--color-accent-red)';
  if (p >= 75) return 'var(--color-accent)';
  return 'var(--color-accent)';
});

// 计算流量状态
const trafficStatus = computed(() => {
  const p = safePercentage.value;
  if (p >= 90) return { text: 'Critical', color: 'var(--color-accent-red)' };
  if (p >= 75) return { text: 'High Traffic', color: 'var(--color-accent)' };
  if (p >= 50) return { text: 'Moderate', color: 'var(--color-text-secondary)' };
  return { text: 'Low Traffic', color: 'var(--color-accent-green)' };
});

// 格式化重置时间
const resetTimeDisplay = computed(() => {
  if (!props.data.resetTime) return null;

  try {
    const resetDate = new Date(props.data.resetTime);
    // 检查日期是否有效
    if (isNaN(resetDate.getTime())) return null;

    const now = new Date();
    const diffMs = resetDate.getTime() - now.getTime();

    if (diffMs <= 0) return 'Reset soon';

    const hours = Math.floor(diffMs / (1000 * 60 * 60));
    const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));

    if (hours > 24) {
      const days = Math.floor(hours / 24);
      return `Reset in ${days}d ${hours % 24}h`;
    }

    return `Reset in ${hours}h ${minutes}m`;
  } catch {
    return null;
  }
});
</script>

<template>
  <div class="usage-card">
    <div class="card-header">
      <div class="card-title-row">
        <!-- 图标 -->
        <div class="card-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M3 12h4l3-9 4 18 3-9h4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <span class="card-title">{{ pluginName }}</span>
      </div>
      <!-- 插件标签 -->
      <span v-if="pluginId" class="plugin-tag">plugin:{{ pluginId }}</span>
    </div>

    <div class="card-content">
      <!-- 大数字显示 -->
      <div class="usage-value">
        <span class="percentage">{{ Math.round(safePercentage) }}%</span>
        <span class="unit">used</span>
      </div>

      <!-- 进度条 -->
      <div class="progress-bar">
        <div
          class="progress-fill"
          :style="{
            width: `${safePercentage}%`,
            backgroundColor: progressColor
          }"
        ></div>
      </div>

      <!-- 底部信息 -->
      <div class="card-footer">
        <span v-if="resetTimeDisplay" class="reset-time">{{ resetTimeDisplay }}</span>
        <span
          class="traffic-status"
          :style="{ color: trafficStatus.color }"
        >
          {{ trafficStatus.text }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.usage-card {
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-md);
  position: relative;
  overflow: hidden;
}

/* 右上角装饰 */
.usage-card::after {
  content: '';
  position: absolute;
  top: -20px;
  right: -20px;
  width: 100px;
  height: 100px;
  background: var(--color-accent);
  opacity: 0.1;
  border-radius: 50%;
  transform: rotate(45deg);
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: var(--spacing-md);
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.card-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-accent);
  border-radius: var(--radius-md);
  color: white;
}

.card-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.plugin-tag {
  font-size: 0.625rem;
  padding: 2px 8px;
  background: var(--color-bg);
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
}

.card-content {
  position: relative;
  z-index: 1;
}

.usage-value {
  display: flex;
  align-items: baseline;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.percentage {
  font-size: 2.5rem;
  font-weight: 700;
  color: var(--color-text);
  line-height: 1;
}

.unit {
  font-size: 1rem;
  color: var(--color-text-secondary);
}

.progress-bar {
  height: 6px;
  background: var(--color-bg);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: var(--spacing-md);
}

.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease, background-color 0.3s ease;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.75rem;
}

.reset-time {
  color: var(--color-text-secondary);
}

.traffic-status {
  font-weight: 500;
}
</style>
