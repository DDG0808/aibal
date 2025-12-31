<script setup lang="ts">
/**
 * Phase 8.1: 托盘弹窗主视图
 * 集成所有卡片组件，显示插件配额数据
 * 支持浏览器 fallback（开发调试用）
 */
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { TrayHeader, UsageCard, StatusCard, PluginBar } from '@/components/tray';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 安全的 Tauri API 调用
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    console.info(`[Mock] invoke('${cmd}')`, args);
    return getMockResult(cmd) as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

async function safeListen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void> {
  if (!isTauri) {
    console.info(`[Mock] listen('${event}')`);
    return () => {};
  }
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, handler);
}

async function safeEmit(event: string, payload?: unknown): Promise<void> {
  if (!isTauri) {
    console.info(`[Mock] emit('${event}')`, payload);
    return;
  }
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const currentWindow = getCurrentWindow();
  await currentWindow.emit(event, payload);
}

// 模拟数据 - 配额维度
interface QuotaDimension {
  name: string;
  percentage: number;
  status: 'available' | 'error' | 'warning';
  resetLabel?: string;
}

interface PluginWithQuotas {
  id: string;
  name: string;
  enabled: boolean;
  healthy: boolean;
  dimensions: QuotaDimension[];
}

// 模拟数据
function getMockResult(cmd: string): unknown {
  switch (cmd) {
    case 'plugin_list':
      return { success: true, data: [
        {
          id: 'antigravity',
          name: 'Antigravity',
          enabled: true,
          healthy: true,
          dimensions: [
            { name: 'Claude Sonnet 4.5', percentage: 5, status: 'available', resetLabel: '15分钟后重置' },
            { name: 'Claude Opus 4.5 (Thinking)', percentage: 5, status: 'error', resetLabel: '15分钟后重置' },
            { name: 'Claude Sonnet 4.5 (Thinking)', percentage: 5, status: 'available', resetLabel: '15分钟后重置' },
          ]
        },
        {
          id: 'openai-tracker',
          name: 'OpenAI',
          enabled: true,
          healthy: true,
          dimensions: [
            { name: 'GPT-4o', percentage: 25, status: 'available', resetLabel: '1小时后重置' },
            { name: 'GPT-4', percentage: 10, status: 'available', resetLabel: '1小时后重置' },
          ]
        },
      ]};
    case 'get_all_data':
      return { success: true, data: [] };
    case 'get_all_health':
      return { success: true, data: [
        { pluginId: 'antigravity', status: 'healthy', successRate: 0.99, lastCheck: new Date().toISOString() },
        { pluginId: 'openai-tracker', status: 'healthy', successRate: 0.99, lastCheck: new Date().toISOString() },
      ]};
    case 'refresh_all':
      return { success: true, data: [] };
    default:
      return { success: true, data: null };
  }
}

import type {
  Result,
  PluginHealth,
  HealthStatus,
} from '@/types';

// 状态
const isRefreshing = ref(false);
const plugins = ref<PluginWithQuotas[]>([]);
const pluginHealth = ref<PluginHealth[]>([]);
const selectedPluginId = ref<string>('');
const error = ref<string | null>(null);

// 事件监听器清理函数
const unlisteners: (() => void)[] = [];

// 计算系统整体状态
const systemStatus = computed<HealthStatus>(() => {
  if (pluginHealth.value.length === 0) return 'healthy';

  const unhealthyCount = pluginHealth.value.filter(h => h.status === 'unhealthy').length;
  const degradedCount = pluginHealth.value.filter(h => h.status === 'degraded').length;

  if (unhealthyCount > 0) return 'unhealthy';
  if (degradedCount > 0) return 'degraded';
  return 'healthy';
});

// 健康插件数量
const healthyPluginCount = computed(() => {
  return pluginHealth.value.filter(h => h.status === 'healthy').length;
});

// 已启用的插件列表
const enabledPlugins = computed(() =>
  plugins.value.filter(p => p.enabled)
);

// 当前选中的插件
const selectedPlugin = computed(() => {
  if (!selectedPluginId.value && enabledPlugins.value.length > 0) {
    return enabledPlugins.value[0];
  }
  return enabledPlugins.value.find(p => p.id === selectedPluginId.value) || enabledPlugins.value[0];
});

// 当前插件的配额维度列表
const currentQuotas = computed(() => {
  if (!selectedPlugin.value) return [];
  return selectedPlugin.value.dimensions || [];
});

// 加载数据
const loadData = async () => {
  try {
    const [pluginListResult, allHealthResult] = await Promise.all([
      safeInvoke<Result<PluginWithQuotas[]>>('plugin_list'),
      safeInvoke<Result<PluginHealth[]>>('get_all_health'),
    ]);

    if (pluginListResult.success && pluginListResult.data) {
      plugins.value = pluginListResult.data;
      // 默认选中第一个启用的插件
      if (!selectedPluginId.value && plugins.value.length > 0) {
        const firstEnabled = plugins.value.find(p => p.enabled);
        if (firstEnabled) {
          selectedPluginId.value = firstEnabled.id;
        }
      }
    }

    if (allHealthResult.success && allHealthResult.data) {
      pluginHealth.value = allHealthResult.data;
    }

    error.value = null;
  } catch (e) {
    console.error('加载数据失败:', e);
    error.value = e instanceof Error ? e.message : '加载数据失败';
  }
};

// 刷新数据
const handleRefresh = async () => {
  if (isRefreshing.value) return;

  isRefreshing.value = true;
  try {
    await safeInvoke<Result<unknown>>('refresh_all', { force: true });
    await loadData();
    error.value = null;
  } catch (e) {
    console.error('刷新失败:', e);
    error.value = e instanceof Error ? e.message : '刷新失败';
  } finally {
    isRefreshing.value = false;
  }
};

// 选择插件
const handleSelectPlugin = (pluginId: string) => {
  selectedPluginId.value = pluginId;
};

// 管理插件
const handleManagePlugins = async () => {
  try {
    await safeEmit('open-settings', { tab: 'plugins' });
  } catch (e) {
    console.error('打开插件管理失败:', e);
  }
};

// 监听事件
const setupEventListeners = async () => {
  // 监听插件数据更新
  const unlistenDataUpdated = await safeListen<{ id: string; dimensions: QuotaDimension[] }>(
    'ipc:plugin_data_updated',
    (event) => {
      const { id, dimensions } = event.payload;
      const plugin = plugins.value.find(p => p.id === id);
      if (plugin) {
        plugin.dimensions = dimensions;
      }
    }
  );
  unlisteners.push(unlistenDataUpdated);

  // 监听健康状态变化
  const unlistenHealthChanged = await safeListen<PluginHealth>(
    'ipc:plugin_health_changed',
    (event) => {
      const health = event.payload;
      const index = pluginHealth.value.findIndex(
        h => h.pluginId === health.pluginId
      );
      if (index >= 0) {
        pluginHealth.value[index] = health;
      } else {
        pluginHealth.value.push(health);
      }
    }
  );
  unlisteners.push(unlistenHealthChanged);
};

// 监听插件变化，自动选中有效插件
watch(enabledPlugins, (newPlugins) => {
  const firstPlugin = newPlugins[0];
  if (firstPlugin && !newPlugins.find(p => p.id === selectedPluginId.value)) {
    selectedPluginId.value = firstPlugin.id;
  }
});

// 生命周期
onMounted(async () => {
  await loadData();
  await setupEventListeners();
});

onUnmounted(() => {
  unlisteners.forEach(unlisten => unlisten());
});
</script>

<template>
  <div class="home-view">
    <!-- 头部：插件选择器 -->
    <TrayHeader
      :plugins="enabledPlugins"
      :selected-plugin-id="selectedPluginId"
      :is-refreshing="isRefreshing"
      :system-status="systemStatus"
      @select-plugin="handleSelectPlugin"
      @refresh="handleRefresh"
    />

    <!-- 主内容区域 -->
    <main class="home-content">
      <!-- 错误提示 -->
      <div
        v-if="error"
        class="error-banner"
      >
        <span>{{ error }}</span>
        <button @click="loadData">
          重试
        </button>
      </div>

      <!-- 配额列表 -->
      <div class="quota-list">
        <UsageCard
          v-for="quota in currentQuotas"
          :key="quota.name"
          :item="quota"
        />

        <!-- 空状态 -->
        <div
          v-if="currentQuotas.length === 0"
          class="empty-state"
        >
          <p class="empty-state-text">
            暂无配额数据
          </p>
          <p class="empty-state-hint">
            请先安装并启用插件
          </p>
        </div>
      </div>

      <!-- 系统状态 -->
      <StatusCard
        :healthy-count="healthyPluginCount"
        :total-count="plugins.length"
      />
    </main>

    <!-- 底部管理按钮 -->
    <PluginBar @manage="handleManagePlugins" />
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  background: var(--color-bg);
}

.home-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 0 var(--spacing-md);
  padding-bottom: var(--spacing-sm);
  overflow-y: auto;
}

.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-accent-red);
  color: white;
  border-radius: var(--radius-md);
  font-size: 0.75rem;
  margin-bottom: var(--spacing-md);
}

.error-banner button {
  background: rgba(255, 255, 255, 0.2);
  border: none;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  color: white;
  cursor: pointer;
  font-size: 0.75rem;
}

.error-banner button:hover {
  background: rgba(255, 255, 255, 0.3);
}

.quota-list {
  flex: 1;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl);
  text-align: center;
}

.empty-state-text {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-xs);
}

.empty-state-hint {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
  margin: 0;
}
</style>
