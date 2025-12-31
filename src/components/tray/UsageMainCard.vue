<script setup lang="ts">
/**
 * Phase 8.2: 主用量卡片
 * 显示当前选中插件的用量数据（大号展示）
 * 参考设计: Claude Usage 78% used
 */
import { computed } from 'vue';
import type { PluginData } from '@/types';

interface Props {
  /** 插件数据 */
  data: PluginData | null;
  /** 插件名称 */
  pluginName?: string;
  /** 插件ID */
  pluginId?: string;
}

const props = withDefaults(defineProps<Props>(), {
  pluginName: 'Usage',
  pluginId: '',
});

// 解析用量数据
const usagePercentage = computed(() => {
  if (!props.data || props.data.dataType !== 'usage') return 0;
  const d = props.data as { percentage?: number };
  return Math.round(d.percentage || 0);
});

const resetLabel = computed(() => {
  if (!props.data) return '';
  const d = props.data as { resetLabel?: string };
  return d.resetLabel || '';
});

// 流量状态判断
const trafficStatus = computed(() => {
  const pct = usagePercentage.value;
  if (pct >= 80) return { label: '高负载', color: '#ef4444' };
  if (pct >= 50) return { label: '中等负载', color: '#f97316' };
  return { label: '正常', color: '#22c55e' };
});

// 进度条颜色
const progressColor = computed(() => {
  const pct = usagePercentage.value;
  if (pct >= 80) return '#ef4444';
  if (pct >= 50) return '#f97316';
  return '#22c55e';
});
</script>

<template>
  <div class="usage-main-card">
    <!-- 头部: 图标 + 标题 + 标签 -->
    <div class="card-header">
      <div class="header-left">
        <div class="icon-wrapper">
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
          >
            <path
              d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
              fill="currentColor"
            />
          </svg>
        </div>
        <span class="title">{{ pluginName }}</span>
      </div>
      <span
        v-if="pluginId"
        class="plugin-tag"
      >plugin:{{ pluginId }}</span>
    </div>

    <!-- 主内容: 百分比 -->
    <div class="usage-display">
      <span class="percentage">{{ usagePercentage }}%</span>
      <span class="label">已用</span>
    </div>

    <!-- 进度条 -->
    <div class="progress-bar">
      <div
        class="progress-fill"
        :style="{
          width: `${usagePercentage}%`,
          backgroundColor: progressColor
        }"
      />
    </div>

    <!-- 底部状态 -->
    <div class="card-footer">
      <span
        v-if="resetLabel"
        class="reset-label"
      >{{ resetLabel }}</span>
      <span
        class="traffic-status"
        :style="{ color: trafficStatus.color }"
      >
        {{ trafficStatus.label }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.usage-main-card {
  background: #1e1e1e;
  border-radius: 16px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-wrapper {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: #f97316;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.title {
  font-size: 1rem;
  font-weight: 600;
  color: #ffffff;
}

.plugin-tag {
  font-size: 0.7rem;
  padding: 4px 10px;
  background: #333;
  border-radius: 999px;
  color: #888;
}

.usage-display {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-top: 4px;
}

.percentage {
  font-size: 3rem;
  font-weight: 700;
  color: #ffffff;
  line-height: 1;
}

.label {
  font-size: 1rem;
  color: #888;
  font-weight: 500;
}

.progress-bar {
  height: 8px;
  background: #333;
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.3s ease, background-color 0.3s ease;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 4px;
}

.reset-label {
  font-size: 0.8rem;
  color: #666;
}

.traffic-status {
  font-size: 0.8rem;
  font-weight: 500;
}
</style>
