<script setup lang="ts">
/**
 * 配额行组件
 * 显示单个模型的配额信息：名称、状态、百分比、进度条
 */
import { computed } from 'vue';

interface QuotaItem {
  /** 模型/维度名称 */
  name: string;
  /** 使用百分比 (0-100) */
  percentage: number;
  /** 状态: available/error/warning */
  status?: 'available' | 'error' | 'warning';
  /** 重置时间描述 */
  resetLabel?: string;
}

interface Props {
  /** 配额项 */
  item: QuotaItem;
}

const props = defineProps<Props>();

// 安全的百分比值 (clamp 到 0-100)
const safePercentage = computed(() => {
  const p = props.item.percentage;
  if (typeof p !== 'number' || isNaN(p)) return 0;
  return Math.max(0, Math.min(100, p));
});

// 剩余百分比
const remainingPercentage = computed(() => 100 - safePercentage.value);

// 计算进度条颜色
const progressColor = computed(() => {
  if (props.item.status === 'error') return 'var(--color-accent-red)';
  const p = safePercentage.value;
  if (p >= 90) return 'var(--color-accent-red)';
  if (p >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
});

// 状态标签配置
const statusConfig = computed(() => {
  switch (props.item.status) {
    case 'error':
      return { text: '错误', class: 'status-error' };
    case 'warning':
      return { text: '警告', class: 'status-warning' };
    case 'available':
    default:
      return { text: '可用', class: 'status-available' };
  }
});
</script>

<template>
  <div class="quota-row">
    <!-- 头部：名称 + 状态 + 百分比 -->
    <div class="quota-header">
      <span class="quota-name">{{ item.name }}</span>
      <div class="quota-right">
        <span
          class="status-tag"
          :class="statusConfig.class"
        >{{ statusConfig.text }}</span>
        <span class="quota-percentage">{{ Math.round(safePercentage) }}%</span>
      </div>
    </div>

    <!-- 进度条 -->
    <div class="progress-bar">
      <div
        class="progress-fill"
        :style="{
          width: `${safePercentage}%`,
          backgroundColor: progressColor
        }"
      />
    </div>

    <!-- 底部信息 -->
    <div class="quota-footer">
      <span class="remaining-info">剩余 {{ Math.round(remainingPercentage) }}%</span>
      <span
        v-if="item.resetLabel"
        class="reset-info"
      >{{ item.resetLabel }}</span>
    </div>
  </div>
</template>

<style scoped>
.quota-row {
  padding: var(--spacing-sm) 0;
}

.quota-row + .quota-row {
  border-top: 1px solid var(--color-border);
}

.quota-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-xs);
}

.quota-name {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.quota-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.status-tag {
  font-size: 0.625rem;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  font-weight: 500;
}

.status-available {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
}

.status-error {
  background: rgba(239, 68, 68, 0.15);
  color: var(--color-accent-red);
}

.status-warning {
  background: rgba(245, 158, 11, 0.15);
  color: var(--color-accent);
}

.quota-percentage {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text);
  min-width: 36px;
  text-align: right;
}

.progress-bar {
  height: 4px;
  background: var(--color-bg-secondary);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: var(--spacing-xs);
}

.progress-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease, background-color 0.3s ease;
}

.quota-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.6875rem;
  color: var(--color-text-tertiary);
}

.remaining-info {
  color: var(--color-accent-green);
}

.reset-info {
  color: var(--color-text-tertiary);
}
</style>
