<script setup lang="ts">
/**
 * Phase 8.1: 托盘弹窗主视图
 * 集成所有卡片组件，显示插件数据和状态
 * 支持浏览器 fallback（开发调试用）
 */
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { TrayHeader, UsageCard, BalanceCard, StatusCard, PluginBar } from '@/components/tray';

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
    return () => {}; // 空的取消监听函数
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

// 模拟数据
function getMockResult(cmd: string): unknown {
  switch (cmd) {
    case 'plugin_list':
      return { success: true, data: [
        { id: 'claude-usage', name: 'Claude Usage', version: '1.0.0', enabled: true, healthy: true, dataType: 'usage' },
      ]};
    case 'get_all_data':
      return { success: true, data: [
        { pluginId: 'claude-usage', dataType: 'usage', percentage: 42, used: 420, limit: 1000, unit: 'msgs', resetLabel: '1h 后重置', lastUpdated: new Date().toISOString(), dimensions: [] },
      ]};
    case 'get_all_health':
      return { success: true, data: [
        { pluginId: 'claude-usage', status: 'healthy', successRate: 0.99, lastCheck: new Date().toISOString() },
      ]};
    case 'refresh_all':
      return { success: true, data: [] };
    case 'get_version':
      return '2.2 (Browser)';
    default:
      return { success: true, data: null };
  }
}
import type {
  Result,
  PluginInfo,
  PluginData,
  UsageData,
  BalanceData,
  StatusData,
  PluginHealth,
  HealthStatus,
  StatusIndicator,
} from '@/types';

// 状态
const isRefreshing = ref(false);
const version = ref('2.2');
const plugins = ref<PluginInfo[]>([]);
const pluginData = ref<PluginData[]>([]);
const pluginHealth = ref<PluginHealth[]>([]);
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

// 分类数据
const usageDataList = computed(() =>
  pluginData.value.filter((d): d is UsageData => d.dataType === 'usage')
);

const balanceDataList = computed(() =>
  pluginData.value.filter((d): d is BalanceData => d.dataType === 'balance')
);

const statusDataList = computed(() =>
  pluginData.value.filter((d): d is StatusData => d.dataType === 'status')
);

// 获取插件名称
const getPluginName = (pluginId: string): string => {
  const plugin = plugins.value.find(p => p.id === pluginId);
  return plugin?.name || pluginId;
};

// 获取插件健康状态
const getPluginHealthStatus = (pluginId: string): HealthStatus => {
  const health = pluginHealth.value.find(h => h.pluginId === pluginId);
  return health?.status || 'healthy';
};

// 获取系统状态指示器
const systemStatusIndicator = computed<StatusIndicator>(() => {
  const firstStatus = statusDataList.value[0];
  if (firstStatus) {
    // 使用第一个状态插件的数据
    return firstStatus.indicator;
  }
  // 根据系统状态计算
  switch (systemStatus.value) {
    case 'healthy': return 'none';
    case 'degraded': return 'minor';
    case 'unhealthy': return 'major';
    default: return 'unknown';
  }
});

// 加载数据
const loadData = async () => {
  try {
    // 并行加载所有数据
    const [pluginListResult, allDataResult, allHealthResult, versionResult] = await Promise.all([
      safeInvoke<Result<PluginInfo[]>>('plugin_list'),
      safeInvoke<Result<PluginData[]>>('get_all_data'),
      safeInvoke<Result<PluginHealth[]>>('get_all_health'),
      safeInvoke<string>('get_version').catch(() => '2.2'),
    ]);

    if (pluginListResult.success && pluginListResult.data) {
      plugins.value = pluginListResult.data;
    }

    if (allDataResult.success && allDataResult.data) {
      pluginData.value = allDataResult.data;
    }

    if (allHealthResult.success && allHealthResult.data) {
      pluginHealth.value = allHealthResult.data;
    }

    version.value = versionResult;
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
    const result = await safeInvoke<Result<PluginData[]>>('refresh_all', { force: true });
    if (result.success && result.data) {
      pluginData.value = result.data;
    }
    // 重新加载健康状态
    const healthResult = await safeInvoke<Result<PluginHealth[]>>('get_all_health');
    if (healthResult.success && healthResult.data) {
      pluginHealth.value = healthResult.data;
    }
    error.value = null;
  } catch (e) {
    console.error('刷新失败:', e);
    error.value = e instanceof Error ? e.message : '刷新失败';
  } finally {
    isRefreshing.value = false;
  }
};

// 打开设置窗口
const handleSettings = async () => {
  try {
    await safeEmit('open-settings');
  } catch (e) {
    console.error('打开设置失败:', e);
  }
};

// 管理插件
const handleManagePlugins = async () => {
  try {
    await safeEmit('open-settings', { tab: 'plugins' });
  } catch (e) {
    console.error('打开插件管理失败:', e);
  }
};

// 插件点击
const handlePluginClick = (plugin: PluginInfo) => {
  console.log('Plugin clicked:', plugin.id);
};

// 监听事件
const setupEventListeners = async () => {
  // 监听插件数据更新 (契约: PluginDataUpdatedEvent)
  const unlistenDataUpdated = await safeListen<{ id: string; data: PluginData }>(
    'ipc:plugin_data_updated',
    (event) => {
      const { id, data } = event.payload;
      // 按 pluginId 去重更新
      const index = pluginData.value.findIndex(
        d => d.pluginId === id
      );
      if (index >= 0) {
        pluginData.value[index] = data;
      } else {
        pluginData.value.push(data);
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

// 生命周期
onMounted(async () => {
  await loadData();
  await setupEventListeners();
});

onUnmounted(() => {
  // 清理事件监听器
  unlisteners.forEach(unlisten => unlisten());
});
</script>

<template>
  <div class="home-view">
    <!-- 头部 -->
    <TrayHeader
      :version="version"
      :is-refreshing="isRefreshing"
      :system-status="systemStatus"
      @refresh="handleRefresh"
      @settings="handleSettings"
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

      <!-- 使用量卡片 -->
      <UsageCard
        v-for="usage in usageDataList"
        :key="usage.pluginId"
        :data="usage"
        :plugin-name="getPluginName(usage.pluginId)"
        :plugin-id="usage.pluginId"
      />

      <!-- 空状态: 没有使用量数据时显示提示 -->
      <div
        v-if="usageDataList.length === 0"
        class="empty-state"
      >
        <div class="empty-state-icon">
          📊
        </div>
        <p class="empty-state-text">
          暂无使用量数据
        </p>
        <p class="empty-state-hint">
          请先安装并启用插件
        </p>
      </div>

      <!-- 余额卡片组 -->
      <div
        v-if="balanceDataList.length > 0"
        class="balance-row"
      >
        <BalanceCard
          v-for="(balance, index) in balanceDataList"
          :key="balance.pluginId"
          :data="balance"
          :plugin-name="getPluginName(balance.pluginId)"
          :health-status="getPluginHealthStatus(balance.pluginId)"
          :color-theme="['green', 'blue', 'orange', 'purple'][index % 4] as 'green' | 'blue' | 'orange' | 'purple'"
        />
      </div>

      <!-- 空状态: 没有余额数据时显示提示 -->
      <div
        v-else
        class="empty-state"
      >
        <div class="empty-state-icon">
          💰
        </div>
        <p class="empty-state-text">
          暂无余额数据
        </p>
        <p class="empty-state-hint">
          安装余额类插件后将在此显示
        </p>
      </div>

      <!-- 系统状态卡片 -->
      <StatusCard
        :indicator="systemStatusIndicator"
        description="System Status"
        :monitored-count="plugins.length"
        expandable
      />
    </main>

    <!-- 底部插件栏（使用真实数据，空时显示空状态） -->
    <PluginBar
      :plugins="plugins"
      @manage="handleManagePlugins"
      @plugin-click="handlePluginClick"
    />
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
  gap: var(--spacing-md);
  padding: 0 var(--spacing-md);
  padding-bottom: var(--spacing-md);
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

.balance-row {
  display: flex;
  gap: var(--spacing-md);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl);
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  text-align: center;
}

.empty-state-icon {
  font-size: 2rem;
  margin-bottom: var(--spacing-sm);
  opacity: 0.6;
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
