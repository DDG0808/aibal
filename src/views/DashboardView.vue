<script setup lang="ts">
/**
 * 仪表盘视图
 * Phase 8.3: 数据聚合展示、健康状态展示
 */
import { ref, computed, onMounted } from 'vue';
import { AppLayout } from '@/components/layout';
import type { UsageData, PluginInfo, PluginHealth } from '@/types';

// 模拟数据 (实际应从 store 获取)
const isLoading = ref(false);
const selectedPluginId = ref('claude-usage');

// 模拟插件列表
const plugins = ref<PluginInfo[]>([
  {
    id: 'claude-usage',
    name: 'Claude 用量监控',
    version: '1.2.0',
    pluginType: 'data',
    dataType: 'usage',
    enabled: true,
    healthy: true,
    author: 'CUK Official',
    description: '监控 Claude Pro 用量与限制',
    icon: 'bolt',
  },
]);

// 模拟使用量数据
const usageData = ref<UsageData>({
  pluginId: 'claude-usage',
  lastUpdated: new Date().toISOString(),
  dataType: 'usage',
  percentage: 78,
  used: 78,
  limit: 100,
  unit: 'msgs',
  resetTime: new Date(Date.now() + 2 * 60 * 60 * 1000 + 15 * 60 * 1000).toISOString(),
  resetLabel: '2小时15分后重置',
  dimensions: [
    {
      id: 'session_5h',
      label: '5小时会话限制',
      percentage: 78,
      used: 39,
      limit: 50,
      resetTime: new Date(Date.now() + 2 * 60 * 60 * 1000 + 15 * 60 * 1000).toISOString(),
    },
    {
      id: 'daily',
      label: '每日总上限',
      percentage: 45,
      used: 225,
      limit: 500,
      resetTime: new Date(Date.now() + 14 * 60 * 60 * 1000).toISOString(),
    },
  ],
});

// 模拟健康数据 (预留给后续使用)
const _healthData = ref<PluginHealth>({
  pluginId: 'claude-usage',
  status: 'healthy',
  lastSuccess: new Date().toISOString(),
  errorCount: 0,
  avgLatencyMs: 450,
  successRate: 0.998,
});
void _healthData; // 防止未使用警告

// 计算属性
const selectedPlugin = computed(() => plugins.value.find(p => p.id === selectedPluginId.value));
const progressColor = computed(() => {
  const pct = usageData.value.percentage;
  if (pct >= 90) return 'var(--color-accent-red)';
  if (pct >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
});

// 格式化重置时间
function formatResetTime(isoTime?: string): string {
  if (!isoTime) return '未知';
  const reset = new Date(isoTime);
  const now = new Date();
  const diff = reset.getTime() - now.getTime();
  if (diff <= 0) return '即将重置';
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  if (hours > 0) return `${hours}h ${minutes}m 重置`;
  return `${minutes}m 重置`;
}

// 刷新数据
async function refreshData() {
  isLoading.value = true;
  // TODO: 调用 IPC refresh_plugin
  await new Promise(resolve => setTimeout(resolve, 1000));
  isLoading.value = false;
}

onMounted(() => {
  // TODO: 从 store 加载数据
});
</script>

<template>
  <AppLayout>
    <template #title>
      <h1>仪表盘</h1>
    </template>

    <div class="dashboard">
      <!-- 主插件卡片 -->
      <div class="plugin-card">
        <div class="card-header">
          <div class="plugin-info">
            <div class="plugin-icon" :style="{ background: 'var(--color-accent)' }">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <div class="plugin-meta">
              <div class="plugin-name-row">
                <span class="plugin-name">{{ selectedPlugin?.name }}</span>
                <svg class="dropdown-icon" width="16" height="16" viewBox="0 0 24 24" fill="none">
                  <path d="M6 9l6 6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
              <div class="plugin-status">
                <span class="status-badge healthy">运行正常</span>
                <span class="update-time">更新于 刚刚</span>
              </div>
            </div>
          </div>
          <button class="refresh-btn" :class="{ loading: isLoading }" @click="refreshData">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path d="M23 4v6h-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M1 20v-6h6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>

        <!-- 主要使用量显示 -->
        <div class="usage-main">
          <div class="usage-stats">
            <span class="usage-label">当前使用量</span>
            <div class="usage-value">
              <span class="percentage">{{ usageData.percentage }}</span>
              <span class="percent-sign">%</span>
            </div>
          </div>
          <div class="usage-meta">
            <div class="reset-badge">{{ usageData.resetLabel }}</div>
            <div class="usage-detail">已用 {{ usageData.used }} / {{ usageData.limit }} {{ usageData.unit }}</div>
          </div>
        </div>

        <!-- 进度条 -->
        <div class="progress-bar">
          <div
            class="progress-fill"
            :style="{ width: usageData.percentage + '%', background: progressColor }"
          ></div>
        </div>

        <!-- 多维度限额详情 -->
        <div class="dimensions-section">
          <div class="section-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <polyline points="14,2 14,8 20,8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <line x1="16" y1="13" x2="8" y2="13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <line x1="16" y1="17" x2="8" y2="17" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>多维度限额详情</span>
          </div>

          <div class="dimensions-grid">
            <div v-for="dim in usageData.dimensions" :key="dim.id" class="dimension-card">
              <div class="dimension-header">
                <span class="dimension-label">{{ dim.label }}</span>
                <span class="dimension-percentage">{{ dim.percentage }}%</span>
              </div>
              <div class="dimension-progress">
                <div
                  class="dimension-progress-fill"
                  :style="{
                    width: dim.percentage + '%',
                    background: dim.percentage >= 75 ? 'var(--color-accent)' : 'var(--color-accent-green)'
                  }"
                ></div>
              </div>
              <div class="dimension-meta">
                <span>{{ dim.used }}/{{ dim.limit }}</span>
                <span>{{ formatResetTime(dim.resetTime) }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 连接监控 -->
        <div class="monitoring-section">
          <div class="section-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <rect x="2" y="3" width="20" height="14" rx="2" stroke="currentColor" stroke-width="2"/>
              <line x1="8" y1="21" x2="16" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <line x1="12" y1="17" x2="12" y2="21" stroke="currentColor" stroke-width="2"/>
            </svg>
            <span>连接监控 (RELIABILITY LAYER)</span>
          </div>
          <!-- 可扩展的监控信息区域 -->
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
.dashboard {
  max-width: 800px;
}

.plugin-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  padding: var(--spacing-xl);
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: var(--spacing-xl);
}

.plugin-info {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.plugin-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.plugin-meta {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  cursor: pointer;
}

.plugin-name {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-text);
}

.dropdown-icon {
  color: var(--color-text-tertiary);
}

.plugin-status {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: 2px var(--spacing-sm);
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 500;
}

.status-badge.healthy {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
}

.status-badge.healthy::before {
  content: '';
  width: 6px;
  height: 6px;
  background: var(--color-accent-green);
  border-radius: 50%;
}

.update-time {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.update-time::before {
  content: '◷ ';
}

.refresh-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-secondary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.refresh-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.refresh-btn.loading svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.usage-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: var(--spacing-md);
}

.usage-stats {
  display: flex;
  flex-direction: column;
}

.usage-label {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-sm);
}

.usage-value {
  display: flex;
  align-items: baseline;
}

.percentage {
  font-size: 4rem;
  font-weight: 700;
  color: var(--color-text);
  line-height: 1;
}

.percent-sign {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-left: var(--spacing-xs);
}

.usage-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--spacing-sm);
}

.reset-badge {
  background: var(--color-accent);
  color: white;
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  font-weight: 500;
}

.usage-detail {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
}

.progress-bar {
  height: 12px;
  background: var(--color-bg-tertiary);
  border-radius: 6px;
  overflow: hidden;
  margin-bottom: var(--spacing-xl);
}

.progress-fill {
  height: 100%;
  border-radius: 6px;
  transition: width var(--transition-normal);
}

.dimensions-section,
.monitoring-section {
  margin-top: var(--spacing-xl);
  padding-top: var(--spacing-xl);
  border-top: 1px solid var(--color-border);
}

.section-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  color: var(--color-text-secondary);
  font-size: 0.8125rem;
  margin-bottom: var(--spacing-lg);
}

.dimensions-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-md);
}

.dimension-card {
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
}

.dimension-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-sm);
}

.dimension-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.dimension-percentage {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text);
}

.dimension-progress {
  height: 6px;
  background: var(--color-bg-secondary);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: var(--spacing-sm);
}

.dimension-progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width var(--transition-normal);
}

.dimension-meta {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}
</style>
