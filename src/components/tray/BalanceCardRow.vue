<script setup lang="ts">
/**
 * Phase 8.2: 余额卡片行
 * 显示两个并排的余额/API卡片
 * 参考设计: OpenAI API $12.45, DeepSeek Y45
 */
import type { PluginData, PluginHealth } from '@/types';

interface BalanceItem {
  pluginId: string;
  name: string;
  value: string;
  currency?: string;
  status: 'healthy' | 'degraded' | 'unhealthy';
  statusLabel: string;
}

interface Props {
  /** 插件数据列表 */
  pluginData: PluginData[];
  /** 健康状态列表 */
  pluginHealth: PluginHealth[];
  /** 插件名称映射 */
  pluginNames: Record<string, string>;
}

const props = defineProps<Props>();

// 构建余额项列表
const balanceItems = computed<BalanceItem[]>(() => {
  return props.pluginData
    .filter(d => d.dataType === 'balance')
    .slice(0, 2) // 最多显示2个
    .map(data => {
      const health = props.pluginHealth.find(h => h.pluginId === data.pluginId);
      const balanceData = data as {
        balance?: number;
        currency?: string;
        limits?: Array<{ remaining?: number; label?: string }>;
      };

      let value = '--';
      let currency = '';

      if (balanceData.limits && balanceData.limits.length > 0) {
        const limit = balanceData.limits[0]!;
        value = typeof limit.remaining === 'number'
          ? limit.remaining.toFixed(2)
          : '--';
      } else if (typeof balanceData.balance === 'number') {
        const curr = balanceData.currency || 'USD';
        currency = curr;
        if (curr === 'CNY') {
          value = `¥${balanceData.balance.toFixed(2)}`;
        } else if (curr === 'USD') {
          value = `$${balanceData.balance.toFixed(2)}`;
        } else {
          value = `${balanceData.balance.toFixed(2)}`;
        }
      }

      return {
        pluginId: data.pluginId,
        name: props.pluginNames[data.pluginId] || data.pluginId,
        value,
        currency,
        status: health?.status || 'healthy',
        statusLabel: health?.status === 'healthy' ? 'API 正常' : 'API 异常',
      };
    });
});

import { computed } from 'vue';
</script>

<template>
  <div class="balance-card-row">
    <div
      v-for="item in balanceItems"
      :key="item.pluginId"
      class="balance-card"
    >
      <div class="card-header">
        <span class="card-name">{{ item.name }}</span>
        <div class="card-icon">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
          >
            <rect
              x="2"
              y="6"
              width="20"
              height="12"
              rx="2"
              stroke="currentColor"
              stroke-width="2"
            />
            <path
              d="M6 12h4"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </div>
      </div>
      <div class="card-value">
        {{ item.value }}
      </div>
      <div
        class="card-status"
        :class="item.status"
      >
        <span class="status-dot" />
        <span class="status-label">{{ item.statusLabel }}</span>
      </div>
    </div>

    <!-- 空状态：少于2个卡片时填充 -->
    <div
      v-for="i in Math.max(0, 2 - balanceItems.length)"
      :key="`empty-${i}`"
      class="balance-card empty"
    >
      <div class="card-header">
        <span class="card-name">{{ i === 1 ? 'API 余额' : 'API 余额' }}</span>
        <div class="card-icon empty-icon">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
          >
            <rect
              x="2"
              y="6"
              width="20"
              height="12"
              rx="2"
              stroke="currentColor"
              stroke-width="2"
            />
            <path
              d="M6 12h4"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </div>
      </div>
      <div class="card-value">
        --
      </div>
      <div class="card-status empty-status">
        <span class="status-dot" />
        <span class="status-label">暂无数据</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.balance-card-row {
  display: flex;
  gap: 12px;
}

.balance-card {
  flex: 1;
  background: #1e1e1e;
  border-radius: 16px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.balance-card.empty {
  opacity: 1;
}

.balance-card.empty .card-name {
  color: #555;
}

.balance-card.empty .card-value {
  color: #444;
}

.empty-icon {
  color: #444 !important;
}

.empty-status {
  color: #555 !important;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-name {
  font-size: 0.85rem;
  color: #888;
}

.card-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #22c55e;
}

.card-value {
  font-size: 1.75rem;
  font-weight: 700;
  color: #ffffff;
  line-height: 1.2;
}

.card-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
}

.card-status.healthy {
  color: #22c55e;
}

.card-status.degraded {
  color: #f97316;
}

.card-status.unhealthy {
  color: #ef4444;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.status-label {
  font-weight: 500;
}
</style>
