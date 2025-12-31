<script setup lang="ts">
/**
 * 余额卡片组件
 * 显示 API 余额、货币和状态
 */
import { computed } from 'vue';
import type { BalanceData, HealthStatus } from '@/types';

interface Props {
  /** 余额数据 */
  data: BalanceData;
  /** 插件名称 */
  pluginName?: string;
  /** 健康状态 */
  healthStatus?: HealthStatus;
  /** 卡片颜色主题 */
  colorTheme?: 'green' | 'blue' | 'orange' | 'purple';
}

const props = withDefaults(defineProps<Props>(), {
  pluginName: 'API Balance',
  healthStatus: 'healthy',
  colorTheme: 'green',
});

// 货币符号映射
const currencySymbols: Record<string, string> = {
  USD: '$',
  EUR: '\u20AC',
  GBP: '\u00A3',
  CNY: '\u00A5',
  JPY: '\u00A5',
};

// 格式化余额显示
const formattedBalance = computed(() => {
  const symbol = currencySymbols[props.data.currency] || props.data.currency;
  const balance = props.data.balance;

  // 小数处理
  if (balance >= 1000) {
    return `${symbol}${balance.toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
  }
  return `${symbol}${balance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
});

// 状态文本和颜色
const statusDisplay = computed(() => {
  switch (props.healthStatus) {
    case 'healthy':
      return { text: 'API Active', color: 'var(--color-accent-green)' };
    case 'degraded':
      return { text: 'Degraded', color: 'var(--color-accent)' };
    case 'unhealthy':
      return { text: 'Inactive', color: 'var(--color-accent-red)' };
    default:
      return { text: 'Unknown', color: 'var(--color-text-secondary)' };
  }
});

// 主题颜色
const themeColor = computed(() => {
  switch (props.colorTheme) {
    case 'green': return '#22c55e';
    case 'blue': return '#3b82f6';
    case 'orange': return '#f97316';
    case 'purple': return '#8b5cf6';
    default: return '#22c55e';
  }
});
</script>

<template>
  <div class="balance-card">
    <div class="card-header">
      <span class="card-title">{{ pluginName }}</span>
      <!-- 颜色标识 -->
      <div
        class="color-badge"
        :style="{ backgroundColor: themeColor }"
      />
    </div>

    <div class="card-content">
      <!-- 余额显示 -->
      <div class="balance-value">
        {{ formattedBalance }}
      </div>

      <!-- 状态指示 -->
      <div class="status-row">
        <span
          class="status-dot"
          :style="{ backgroundColor: statusDisplay.color }"
        />
        <span
          class="status-text"
          :style="{ color: statusDisplay.color }"
        >
          {{ statusDisplay.text }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.balance-card {
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-md);
  flex: 1;
  min-width: 0;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-sm);
}

.card-title {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  font-weight: 500;
}

.color-badge {
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  opacity: 0.8;
}

.card-content {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.balance-value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--color-text);
  line-height: 1.2;
}

.status-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-text {
  font-size: 0.75rem;
  font-weight: 500;
}
</style>
