<script setup lang="ts">
/**
 * 仪表盘视图
 * Phase 8.3: 数据聚合展示、健康状态展示
 */
import { ref, computed, onMounted } from 'vue';
import { AppLayout } from '@/components/layout';
import { IconBolt, IconRefresh } from '@/components/icons';
import { usePluginStore } from '@/stores';
import type { UsageData } from '@/types';

const pluginStore = usePluginStore();

// 状态
const isLoading = ref(false);
const selectedPluginId = ref('');
const showPluginDropdown = ref(false);

// 从 Store 获取数据
const plugins = computed(() => pluginStore.plugins.filter(p => p.dataType === 'usage'));
const hasPlugins = computed(() => plugins.value.length > 0);
const selectedPlugin = computed(() => plugins.value.find(p => p.id === selectedPluginId.value));
const healthData = computed(() => pluginStore.pluginHealth.get(selectedPluginId.value));

// 插件下拉框
function toggleDropdown() {
  if (plugins.value.length > 1) {
    showPluginDropdown.value = !showPluginDropdown.value;
  }
}

function selectPlugin(id: string) {
  selectedPluginId.value = id;
  showPluginDropdown.value = false;
}

// 跳转到市场
function goToMarketplace() {
  window.location.href = '#/marketplace';
}

// 获取使用量数据
const usageData = computed<UsageData>(() => {
  const data = pluginStore.pluginData.get(selectedPluginId.value);
  if (data && data.dataType === 'usage') {
    return data as UsageData;
  }
  // 默认空数据
  return {
    pluginId: selectedPluginId.value || 'unknown',
    lastUpdated: new Date().toISOString(),
    dataType: 'usage',
    percentage: 0,
    used: 0,
    limit: 100,
    unit: 'msgs',
    resetTime: '',
    resetLabel: '--',
    dimensions: [],
  };
});

// 进度条颜色
const progressColor = computed(() => {
  const pct = usageData.value.percentage;
  if (pct >= 90) return 'var(--color-accent-red)';
  if (pct >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
});

// 健康状态（无数据时显示 unknown 而非 healthy）
const healthStatus = computed(() => {
  if (!healthData.value) return 'unknown';
  return healthData.value.status;
});

const healthLabel = computed(() => {
  switch (healthStatus.value) {
    case 'healthy': return '运行正常';
    case 'degraded': return '性能降级';
    case 'unhealthy': return '运行异常';
    case 'unknown': return '状态未知';
    default: return '未知';
  }
});

// 格式化更新时间
const updateTimeLabel = computed(() => {
  const data = pluginStore.pluginData.get(selectedPluginId.value);
  if (!data?.lastUpdated) return '未更新';
  const diff = Date.now() - new Date(data.lastUpdated).getTime();
  if (diff < 60000) return '刚刚';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`;
  return `${Math.floor(diff / 3600000)}小时前`;
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
  if (!selectedPluginId.value) return;
  isLoading.value = true;
  try {
    await pluginStore.refreshPlugin(selectedPluginId.value, true);
  } finally {
    isLoading.value = false;
  }
}

// 初始化
onMounted(async () => {
  // 始终调用 init 确保 plugins/data/health 都已加载
  await pluginStore.init();
  // 只选择 usage 类型插件（Dashboard 专用于 usage 展示）
  const firstUsagePlugin = plugins.value[0];
  if (firstUsagePlugin) {
    selectedPluginId.value = firstUsagePlugin.id;
  }
  // 若无 usage 插件，selectedPluginId 保持空，UI 会显示空状态
});
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>仪表盘</h2>
    </template>

    <div class="dashboard">
      <!-- 空状态 -->
      <div v-if="!hasPlugins" class="empty-state">
        <div class="empty-icon">📊</div>
        <h3>暂无用量监控插件</h3>
        <p>安装插件后即可在此查看 AI 服务的使用量、余额等数据</p>
        <button class="go-marketplace-btn" @click="goToMarketplace">前往插件市场</button>
      </div>

      <!-- 主插件卡片（有插件时） -->
      <div v-else class="plugin-card">
        <div class="card-header">
          <div class="plugin-info">
            <div
              class="plugin-icon"
              :style="{ background: 'var(--color-accent)' }"
            >
              <IconBolt />
            </div>
            <div class="plugin-meta">
              <div class="plugin-name-row" @click="toggleDropdown">
                <span class="plugin-name">{{ selectedPlugin?.name }}</span>
                <svg
                  v-if="plugins.length > 1"
                  class="dropdown-icon"
                  :class="{ open: showPluginDropdown }"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <path
                    d="M6 9l6 6 6-6"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </div>
              <!-- 插件下拉框 -->
              <div v-if="showPluginDropdown" class="plugin-dropdown">
                <div
                  v-for="plugin in plugins"
                  :key="plugin.id"
                  class="dropdown-item"
                  :class="{ active: plugin.id === selectedPluginId }"
                  @click="selectPlugin(plugin.id)"
                >
                  {{ plugin.name }}
                </div>
              </div>
              <div class="plugin-status">
                <span
                  class="status-badge"
                  :class="healthStatus"
                >{{ healthLabel }}</span>
                <span class="update-time">更新于 {{ updateTimeLabel }}</span>
              </div>
            </div>
          </div>
          <button
            class="refresh-btn"
            :class="{ loading: isLoading }"
            aria-label="刷新数据"
            @click="refreshData"
          >
            <IconRefresh />
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
            <div class="reset-badge">
              {{ usageData.resetLabel }}
            </div>
            <div class="usage-detail">
              已用 {{ usageData.used }} / {{ usageData.limit }} {{ usageData.unit }}
            </div>
          </div>
        </div>

        <!-- 进度条 -->
        <div class="progress-bar">
          <div
            class="progress-fill"
            :style="{ width: usageData.percentage + '%', background: progressColor }"
          />
        </div>

        <!-- 多维度限额详情 -->
        <div class="dimensions-section">
          <div class="section-header">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <polyline
                points="14,2 14,8 20,8"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <line
                x1="16"
                y1="13"
                x2="8"
                y2="13"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <line
                x1="16"
                y1="17"
                x2="8"
                y2="17"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <span>多维度限额详情</span>
          </div>

          <div class="dimensions-grid">
            <div
              v-for="dim in usageData.dimensions"
              :key="dim.id"
              class="dimension-card"
            >
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
                />
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
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <rect
                x="2"
                y="3"
                width="20"
                height="14"
                rx="2"
                stroke="currentColor"
                stroke-width="2"
              />
              <line
                x1="8"
                y1="21"
                x2="16"
                y2="21"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
              <line
                x1="12"
                y1="17"
                x2="12"
                y2="21"
                stroke="currentColor"
                stroke-width="2"
              />
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

/* 空状态 */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-3xl) var(--spacing-xl);
  text-align: center;
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
}

.empty-icon {
  font-size: 4rem;
  margin-bottom: var(--spacing-lg);
}

.empty-state h3 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: var(--spacing-sm);
}

.empty-state p {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-xl);
  max-width: 300px;
}

.go-marketplace-btn {
  background: var(--color-accent);
  color: white;
  border: none;
  padding: var(--spacing-sm) var(--spacing-xl);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.go-marketplace-btn:hover {
  background: var(--color-accent-hover);
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
  position: relative;
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
  transition: transform var(--transition-fast);
}

.dropdown-icon.open {
  transform: rotate(180deg);
}

/* 插件下拉框 */
.plugin-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
  margin-top: var(--spacing-xs);
  overflow: hidden;
}

.dropdown-item {
  padding: var(--spacing-sm) var(--spacing-md);
  cursor: pointer;
  font-size: 0.875rem;
  color: var(--color-text);
  transition: background var(--transition-fast);
}

.dropdown-item:hover {
  background: var(--color-bg-hover);
}

.dropdown-item.active {
  background: var(--color-accent);
  color: white;
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

.status-badge.degraded {
  background: rgba(239, 68, 68, 0.15);
  color: var(--color-accent-red);
}

.status-badge.degraded::before {
  content: '';
  width: 6px;
  height: 6px;
  background: var(--color-accent-red);
  border-radius: 50%;
}

.status-badge.unhealthy {
  background: rgba(239, 68, 68, 0.25);
  color: var(--color-accent-red);
}

.status-badge.unhealthy::before {
  content: '';
  width: 6px;
  height: 6px;
  background: var(--color-accent-red);
  border-radius: 50%;
}

.status-badge.unknown {
  background: rgba(156, 163, 175, 0.15);
  color: var(--color-text-secondary);
}

.status-badge.unknown::before {
  content: '';
  width: 6px;
  height: 6px;
  background: var(--color-text-tertiary);
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
