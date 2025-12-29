<script setup lang="ts">
/**
 * 我的插件视图
 * Phase 8.2: 插件管理、启用/禁用/删除插件
 */
import { ref, computed } from 'vue';
import { useRouter } from 'vue-router';
import { AppLayout } from '@/components/layout';
import type { PluginInfo, HealthStatus } from '@/types';

const router = useRouter();

// 模拟数据
const plugins = ref<(PluginInfo & { calls: number; successRate: number; latency: number; status: HealthStatus })[]>([
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
    calls: 1240,
    successRate: 99.8,
    latency: 450,
    status: 'healthy',
  },
  {
    id: 'openai-balance',
    name: 'OpenAI 余额',
    version: '2.0.1',
    pluginType: 'data',
    dataType: 'balance',
    enabled: true,
    healthy: true,
    author: 'CUK Official',
    description: '追踪 API 信用额度余额',
    icon: 'credit',
    calls: 85,
    successRate: 100,
    latency: 820,
    status: 'healthy',
  },
  {
    id: 'deepseek-monitor',
    name: 'DeepSeek 监控',
    version: '0.9.5',
    pluginType: 'data',
    dataType: 'status',
    enabled: true,
    healthy: false,
    author: 'Community',
    description: 'DeepSeek API 实时状态',
    icon: 'chart',
    calls: 320,
    successRate: 85.2,
    latency: 1200,
    status: 'degraded',
  },
]);

// 统计数据
const stats = computed(() => ({
  totalPlugins: plugins.value.length,
  newPlugins: 2,
  systemHealth: Math.round(plugins.value.reduce((acc, p) => acc + p.successRate, 0) / plugins.value.length),
  totalCalls: plugins.value.reduce((acc, p) => acc + p.calls, 0),
}));

// 切换插件启用状态
function togglePlugin(id: string) {
  const plugin = plugins.value.find(p => p.id === id);
  if (plugin) {
    plugin.enabled = !plugin.enabled;
  }
}

// 跳转到设置页面配置插件 (预留给后续使用)
function _configurePlugin(id: string) {
  router.push({ path: '/settings', query: { plugin: id } });
}
void _configurePlugin; // 防止未使用警告

// 获取状态标签
function getStatusLabel(status: HealthStatus): string {
  switch (status) {
    case 'healthy': return '运行正常';
    case 'degraded': return '性能降级';
    case 'unhealthy': return '运行异常';
    default: return '未知';
  }
}

// 获取状态颜色类
function getStatusClass(status: HealthStatus): string {
  switch (status) {
    case 'healthy': return 'status-healthy';
    case 'degraded': return 'status-degraded';
    case 'unhealthy': return 'status-unhealthy';
    default: return '';
  }
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h1>我的插件</h1>
    </template>

    <div class="plugins-page">
      <!-- 统计概览 -->
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path d="M12 2L2 7l10 5 10-5-10-5z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M2 17l10 5 10-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M2 12l10 5 10-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="stat-content">
            <span class="stat-label">插件总数</span>
            <div class="stat-value">
              <span class="stat-number">{{ stats.totalPlugins }}</span>
              <span class="stat-change positive">+{{ stats.newPlugins }} 新增</span>
            </div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path d="M22 12h-4l-3 9L9 3l-3 9H2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="stat-content">
            <span class="stat-label">系统健康度</span>
            <div class="stat-value">
              <span class="stat-number">{{ stats.systemHealth }}%</span>
              <span class="stat-sublabel">正常运行时间</span>
            </div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <polyline points="23,4 23,10 17,10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="stat-content">
            <span class="stat-label">总调用量</span>
            <div class="stat-value">
              <span class="stat-number">{{ (stats.totalCalls / 10000).toFixed(2) }}万</span>
              <span class="stat-sublabel">今日</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 已安装插件 -->
      <div class="plugins-section">
        <div class="section-header">
          <h2>已安装插件</h2>
          <button class="add-plugin-btn">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            添加插件
          </button>
        </div>

        <div class="plugins-list">
          <div v-for="plugin in plugins" :key="plugin.id" class="plugin-item">
            <div class="plugin-left">
              <div class="plugin-icon" :class="plugin.icon">
                <svg v-if="plugin.icon === 'bolt'" width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                <svg v-else-if="plugin.icon === 'credit'" width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <rect x="1" y="4" width="22" height="16" rx="2" stroke="currentColor" stroke-width="2"/>
                  <line x1="1" y1="10" x2="23" y2="10" stroke="currentColor" stroke-width="2"/>
                </svg>
                <svg v-else width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <path d="M22 12h-4l-3 9L9 3l-3 9H2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>

              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{{ plugin.name }}</span>
                  <span class="plugin-version">v{{ plugin.version }}</span>
                </div>
                <p class="plugin-description">{{ plugin.description }}</p>
                <div class="plugin-stats">
                  <span class="stat">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
                      <path d="M22 12h-4l-3 9L9 3l-3 9H2" stroke="currentColor" stroke-width="2"/>
                    </svg>
                    {{ plugin.calls }} 次调用
                  </span>
                  <span class="stat">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
                      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
                      <path d="M12 6v6l4 2" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                    {{ plugin.successRate }}% 成功率
                  </span>
                  <span class="stat">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
                      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
                      <polyline points="12,6 12,12 16,14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                    {{ plugin.latency }}ms
                  </span>
                </div>
              </div>
            </div>

            <div class="plugin-right">
              <div class="plugin-status" :class="getStatusClass(plugin.status)">
                {{ getStatusLabel(plugin.status) }}
              </div>
              <label class="toggle">
                <input type="checkbox" :checked="plugin.enabled" @change="togglePlugin(plugin.id)">
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
.plugins-page {
  max-width: 900px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--spacing-lg);
  margin-bottom: var(--spacing-xl);
}

.stat-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-md);
}

.stat-icon {
  width: 40px;
  height: 40px;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-secondary);
}

.stat-content {
  flex: 1;
}

.stat-label {
  font-size: 0.8125rem;
  color: var(--color-text-secondary);
  display: block;
  margin-bottom: var(--spacing-xs);
}

.stat-value {
  display: flex;
  align-items: baseline;
  gap: var(--spacing-sm);
}

.stat-number {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--color-text);
}

.stat-change {
  font-size: 0.75rem;
  font-weight: 500;
}

.stat-change.positive {
  color: var(--color-accent-green);
}

.stat-sublabel {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.plugins-section {
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  padding: var(--spacing-xl);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-lg);
}

.section-header h2 {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.add-plugin-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.add-plugin-btn:hover {
  background: var(--color-bg-hover);
  border-color: var(--color-border-light);
}

.plugins-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.plugin-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-lg);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  transition: background var(--transition-fast);
}

.plugin-item:hover {
  background: var(--color-bg-hover);
}

.plugin-left {
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

.plugin-icon.bolt {
  background: var(--color-accent);
}

.plugin-icon.credit {
  background: var(--color-text-tertiary);
}

.plugin-icon.chart {
  background: var(--color-accent-blue);
}

.plugin-info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.plugin-name {
  font-weight: 600;
  color: var(--color-text);
}

.plugin-version {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
  background: var(--color-bg-tertiary);
  padding: 2px var(--spacing-sm);
  border-radius: var(--radius-sm);
}

.plugin-description {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin: 0;
}

.plugin-stats {
  display: flex;
  gap: var(--spacing-lg);
  margin-top: var(--spacing-xs);
}

.plugin-stats .stat {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.plugin-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-lg);
}

.plugin-status {
  font-size: 0.75rem;
  font-weight: 500;
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: 9999px;
}

.plugin-status.status-healthy {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
}

.plugin-status.status-degraded {
  background: rgba(239, 68, 68, 0.15);
  color: var(--color-accent-red);
}

.plugin-status.status-unhealthy {
  background: rgba(239, 68, 68, 0.25);
  color: var(--color-accent-red);
}

/* Toggle Switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-tertiary);
  transition: var(--transition-fast);
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: var(--transition-fast);
  border-radius: 50%;
}

.toggle input:checked + .toggle-slider {
  background-color: var(--color-accent-green);
}

.toggle input:checked + .toggle-slider:before {
  transform: translateX(20px);
}
</style>
