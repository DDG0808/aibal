<script setup lang="ts">
/**
 * Phase 8.1: 托盘弹窗主视图
 * 集成所有卡片组件，显示插件配额数据
 * 支持浏览器 fallback（开发调试用）
 */
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { TrayHeader, UsageCard, PluginBar } from '@/components/tray';
import { useAppStore, usePluginStore } from '@/stores';
import type {
  Result,
  PluginInfo,
  PluginData,
  UsageData,
  PluginHealth,
  HealthStatus,
  UsageDimension,
} from '@/types';

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

// 模拟数据（开发调试用）
function getMockResult(cmd: string): unknown {
  switch (cmd) {
    case 'plugin_list':
      return { success: true, data: [
        { id: 'antigravity', name: 'Antigravity', version: '1.0.0', pluginType: 'data', dataType: 'usage', enabled: true, healthy: true },
        { id: 'openai-tracker', name: 'OpenAI', version: '1.0.0', pluginType: 'data', dataType: 'usage', enabled: true, healthy: true },
      ]};
    case 'get_all_data':
      return { success: true, data: [
        {
          pluginId: 'antigravity',
          dataType: 'usage',
          percentage: 5,
          used: 50,
          limit: 1000,
          unit: 'requests',
          lastUpdated: new Date().toISOString(),
          dimensions: [
            { id: 'sonnet-4.5', label: 'Claude Sonnet 4.5', percentage: 5, used: 50, limit: 1000, resetTime: new Date(Date.now() + 15 * 60 * 1000).toISOString() },
            { id: 'opus-4.5', label: 'Claude Opus 4.5 (Thinking)', percentage: 5, used: 50, limit: 1000, resetTime: new Date(Date.now() + 15 * 60 * 1000).toISOString() },
            { id: 'sonnet-4.5-thinking', label: 'Claude Sonnet 4.5 (Thinking)', percentage: 5, used: 50, limit: 1000, resetTime: new Date(Date.now() + 15 * 60 * 1000).toISOString() },
          ]
        },
      ]};
    case 'get_all_health':
      return { success: true, data: [
        { pluginId: 'antigravity', status: 'healthy', successRate: 0.99, errorCount: 0, avgLatencyMs: 100 },
        { pluginId: 'openai-tracker', status: 'healthy', successRate: 0.99, errorCount: 0, avgLatencyMs: 100 },
      ]};
    case 'refresh_all':
      return { success: true, data: [] };
    default:
      return { success: true, data: null };
  }
}

// 配额行接口（给 UsageCard 组件使用）
interface QuotaItem {
  name: string;
  percentage: number;
  status: 'available' | 'error' | 'warning';
  resetLabel?: string;
}

// Store
const appStore = useAppStore();
const pluginStore = usePluginStore();

// 状态
const isRefreshing = ref(false);
const plugins = ref<PluginInfo[]>([]);
const pluginData = ref<PluginData[]>([]);
const pluginHealth = ref<PluginHealth[]>([]);
const error = ref<string | null>(null);

// 使用 store 中的 selectedPluginId（跨窗口同步）
const selectedPluginId = computed(() => pluginStore.selectedPluginId);

// 深色模式
const isDarkMode = computed(() =>
  appStore.theme === 'dark' ||
  (appStore.theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
);

// 切换主题
const handleToggleTheme = () => {
  if (appStore.theme === 'system') {
    appStore.setTheme(isDarkMode.value ? 'light' : 'dark');
  } else {
    appStore.setTheme(appStore.theme === 'dark' ? 'light' : 'dark');
  }
};

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

// 已启用且有数据类型的插件列表（与仪表盘一致）
const enabledDataPlugins = computed(() => {
  return plugins.value.filter(p => p.enabled && p.dataType);
});

// 当前选中的插件
const selectedPlugin = computed(() => {
  if (!selectedPluginId.value && enabledDataPlugins.value.length > 0) {
    return enabledDataPlugins.value[0];
  }
  return enabledDataPlugins.value.find(p => p.id === selectedPluginId.value) || enabledDataPlugins.value[0];
});

// 当前插件的使用量数据
const currentUsageData = computed<UsageData | null>(() => {
  if (!selectedPlugin.value) return null;
  const data = pluginData.value.find(
    d => d.pluginId === selectedPlugin.value!.id && d.dataType === 'usage'
  );
  return data as UsageData | null;
});

// 获取插件健康状态
const getPluginHealthStatus = (pluginId: string): HealthStatus => {
  const health = pluginHealth.value.find(h => h.pluginId === pluginId);
  return health?.status || 'healthy';
};

// 将 UsageDimension 转换为 QuotaItem
const dimensionToQuotaItem = (dim: UsageDimension, pluginHealthy: boolean): QuotaItem => {
  // 计算重置时间标签
  let resetLabel: string | undefined;
  if (dim.resetTime) {
    try {
      const resetDate = new Date(dim.resetTime);
      const now = new Date();
      const diffMs = resetDate.getTime() - now.getTime();
      if (diffMs > 0) {
        const minutes = Math.floor(diffMs / (1000 * 60));
        if (minutes < 60) {
          resetLabel = `${minutes}分钟后重置`;
        } else {
          const hours = Math.floor(minutes / 60);
          resetLabel = `${hours}小时后重置`;
        }
      }
    } catch {
      // 忽略解析错误
    }
  }

  // 确定状态
  let status: 'available' | 'error' | 'warning' = 'available';
  if (!pluginHealthy) {
    status = 'error';
  } else if (dim.percentage >= 90) {
    status = 'error';
  } else if (dim.percentage >= 75) {
    status = 'warning';
  }

  return {
    name: dim.label,
    percentage: dim.percentage,
    status,
    resetLabel,
  };
};

// 当前插件的配额列表
const currentQuotas = computed<QuotaItem[]>(() => {
  const usageData = currentUsageData.value;
  if (!usageData) return [];

  const pluginHealthy = getPluginHealthStatus(usageData.pluginId) === 'healthy';

  // 如果有 dimensions，使用 dimensions
  if (usageData.dimensions && usageData.dimensions.length > 0) {
    return usageData.dimensions.map(dim => dimensionToQuotaItem(dim, pluginHealthy));
  }

  // 否则使用顶层数据创建单个配额项
  let resetLabel: string | undefined;
  if (usageData.resetLabel) {
    resetLabel = usageData.resetLabel;
  } else if (usageData.resetTime) {
    try {
      const resetDate = new Date(usageData.resetTime);
      const now = new Date();
      const diffMs = resetDate.getTime() - now.getTime();
      if (diffMs > 0) {
        const minutes = Math.floor(diffMs / (1000 * 60));
        if (minutes < 60) {
          resetLabel = `${minutes}分钟后重置`;
        } else {
          const hours = Math.floor(minutes / 60);
          resetLabel = `${hours}小时后重置`;
        }
      }
    } catch {
      // 忽略解析错误
    }
  }

  return [{
    name: selectedPlugin.value?.name || '使用量',
    percentage: usageData.percentage,
    status: pluginHealthy ? (usageData.percentage >= 90 ? 'error' : usageData.percentage >= 75 ? 'warning' : 'available') : 'error',
    resetLabel,
  }];
});

// 加载数据
const loadData = async (force = false) => {
  try {
    // 使用 refresh_all 而非 get_all_data，确保实际执行插件 fetchData 并返回数据
    const [pluginListResult, allDataResult, allHealthResult] = await Promise.all([
      safeInvoke<Result<PluginInfo[]>>('plugin_list'),
      safeInvoke<Result<PluginData[]>>('refresh_all', { force }),
      safeInvoke<Result<PluginHealth[]>>('get_all_health'),
    ]);

    if (pluginListResult.success && pluginListResult.data) {
      plugins.value = pluginListResult.data;
      // 默认选中第一个启用的 usage 插件（如果 store 中还没有选中的）
      if (!pluginStore.selectedPluginId) {
        const firstEnabled = plugins.value.find(p => p.enabled && p.dataType);
        if (firstEnabled) {
          pluginStore.selectPlugin(firstEnabled.id);
        }
      }
    }

    if (allDataResult.success && allDataResult.data) {
      pluginData.value = allDataResult.data;
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

// 刷新数据（强制刷新）
const handleRefresh = async () => {
  if (isRefreshing.value) return;

  isRefreshing.value = true;
  try {
    await loadData(true);
  } catch (e) {
    console.error('刷新失败:', e);
    error.value = e instanceof Error ? e.message : '刷新失败';
  } finally {
    isRefreshing.value = false;
  }
};

// 选择插件（使用 store 方法，会自动同步到其他窗口）
const handleSelectPlugin = (pluginId: string) => {
  pluginStore.selectPlugin(pluginId);
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
  const unlistenDataUpdated = await safeListen<{ id: string; data: PluginData }>(
    'ipc:plugin_data_updated',
    (event) => {
      const { id, data } = event.payload;
      const index = pluginData.value.findIndex(d => d.pluginId === id);
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

  // 监听窗口失焦事件 - 点击外部时自动隐藏弹窗
  if (isTauri) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const currentWindow = getCurrentWindow();
      const unlistenBlur = await currentWindow.onFocusChanged(({ payload: focused }) => {
        if (!focused) {
          // 窗口失去焦点时隐藏
          currentWindow.hide();
        }
      });
      unlisteners.push(unlistenBlur);
    } catch (e) {
      console.warn('设置窗口失焦监听失败:', e);
    }
  }
};

// 监听插件变化，自动选中有效插件
watch(enabledDataPlugins, (newPlugins) => {
  const firstPlugin = newPlugins[0];
  if (firstPlugin && !newPlugins.find(p => p.id === pluginStore.selectedPluginId)) {
    pluginStore.selectPlugin(firstPlugin.id);
  }
});

// 插件选择监听器清理函数
let unlistenPluginSelection: (() => void) | null = null;

// 生命周期
onMounted(async () => {
  // 设置透明背景以支持圆角窗口
  document.body.style.background = 'transparent';
  document.documentElement.style.background = 'transparent';

  await loadData();
  await setupEventListeners();

  // 监听跨窗口的插件选择事件（与仪表盘同步）
  unlistenPluginSelection = await pluginStore.setupPluginSelectionListener();
});

onUnmounted(() => {
  // 恢复背景色
  document.body.style.background = '';
  document.documentElement.style.background = '';

  unlisteners.forEach(unlisten => unlisten());

  // 清理插件选择监听
  if (unlistenPluginSelection) {
    unlistenPluginSelection();
    unlistenPluginSelection = null;
  }
});
</script>

<template>
  <div class="home-view">
    <!-- 头部：插件选择器 + 刷新 + 主题切换 -->
    <TrayHeader
      :plugins="enabledDataPlugins"
      :selected-plugin-id="selectedPluginId"
      :is-refreshing="isRefreshing"
      :system-status="systemStatus"
      :is-dark-mode="isDarkMode"
      @select-plugin="handleSelectPlugin"
      @refresh="handleRefresh"
      @toggle-theme="handleToggleTheme"
    />

    <!-- 主内容区域（可滑动） -->
    <main class="home-content">
      <!-- 错误提示 -->
      <div
        v-if="error"
        class="error-banner"
      >
        <span>{{ error }}</span>
        <button @click="loadData()">
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
    </main>

    <!-- 底部栏：系统状态 + 插件图标 + 管理按钮 -->
    <PluginBar
      :plugins="plugins"
      :healthy-count="healthyPluginCount"
      @manage="handleManagePlugins"
    />
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  background: var(--color-bg);
  border-radius: 12px;
  overflow: hidden;
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
