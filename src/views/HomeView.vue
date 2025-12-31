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
  BalanceData,
  BalanceItem,
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
  used?: number;
  total?: number;
  currency?: string;
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
// 插件执行错误（按插件 ID 存储）
const pluginErrors = ref<Map<string, { code: string; message: string }>>(new Map());

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

// 当前插件的数据（支持 usage 和 balance 类型）
const currentPluginData = computed<PluginData | null>(() => {
  if (!selectedPlugin.value) return null;
  const data = pluginData.value.find(
    d => d.pluginId === selectedPlugin.value!.id
  );
  return data || null;
});

// 当前插件的错误
const currentPluginError = computed<{ code: string; message: string } | null>(() => {
  if (!selectedPlugin.value) return null;
  return pluginErrors.value.get(selectedPlugin.value.id) ?? null;
});

// 获取插件健康状态
const getPluginHealthStatus = (pluginId: string): HealthStatus => {
  const health = pluginHealth.value.find(h => h.pluginId === pluginId);
  return health?.status || 'healthy';
};

// 计算重置时间标签
const formatResetLabel = (resetTime?: string): string | undefined => {
  if (!resetTime) return undefined;
  try {
    const resetDate = new Date(resetTime);
    const now = new Date();
    const diffMs = resetDate.getTime() - now.getTime();
    if (diffMs > 0) {
      const minutes = Math.floor(diffMs / (1000 * 60));
      if (minutes < 60) {
        return `${minutes}分钟后重置`;
      } else {
        const hours = Math.floor(minutes / 60);
        return `${hours}小时后重置`;
      }
    }
  } catch {
    // 忽略解析错误
  }
  return undefined;
};

// 根据百分比确定状态
const getStatusFromPercentage = (percentage: number, pluginHealthy: boolean): 'available' | 'error' | 'warning' => {
  if (!pluginHealthy) return 'error';
  if (percentage >= 90) return 'error';
  if (percentage >= 75) return 'warning';
  return 'available';
};

// 将 UsageDimension 转换为 QuotaItem
const dimensionToQuotaItem = (dim: UsageDimension, pluginHealthy: boolean): QuotaItem => {
  return {
    name: dim.label,
    percentage: dim.percentage,
    used: dim.used,
    total: dim.limit,
    status: getStatusFromPercentage(dim.percentage, pluginHealthy),
    resetLabel: formatResetLabel(dim.resetTime),
  };
};

// 将 BalanceItem 转换为 QuotaItem
const balanceItemToQuotaItem = (item: BalanceItem, pluginHealthy: boolean): QuotaItem => {
  return {
    name: item.name,
    percentage: item.percentage,
    used: item.used,
    total: item.quota,
    currency: item.currency,
    status: getStatusFromPercentage(item.percentage, pluginHealthy),
    resetLabel: item.resetLabel || formatResetLabel(item.resetTime),
  };
};

// 当前插件的配额列表（支持 usage 和 balance 类型）
const currentQuotas = computed<QuotaItem[]>(() => {
  const data = currentPluginData.value;
  if (!data) {
    console.log('[HomeView] currentQuotas: no data');
    return [];
  }

  console.log('[HomeView] currentQuotas: data=', JSON.stringify(data, null, 2));

  const pluginHealthy = getPluginHealthStatus(data.pluginId) === 'healthy';

  // 处理 usage 类型
  if (data.dataType === 'usage') {
    const usageData = data as UsageData;
    // 如果有 dimensions，使用 dimensions
    if (usageData.dimensions && usageData.dimensions.length > 0) {
      return usageData.dimensions.map(dim => dimensionToQuotaItem(dim, pluginHealthy));
    }
    // 否则使用顶层数据创建单个配额项
    return [{
      name: selectedPlugin.value?.name || '使用量',
      percentage: usageData.percentage,
      used: usageData.used,
      total: usageData.limit,
      status: getStatusFromPercentage(usageData.percentage, pluginHealthy),
      resetLabel: usageData.resetLabel || formatResetLabel(usageData.resetTime),
    }];
  }

  // 处理 balance 类型
  if (data.dataType === 'balance') {
    const balanceData = data as BalanceData;
    // 如果有 items，使用 items
    if (balanceData.items && balanceData.items.length > 0) {
      return balanceData.items.map(item => balanceItemToQuotaItem(item, pluginHealthy));
    }
    // 否则使用顶层数据创建单个配额项
    const percentage = balanceData.quota
      ? Math.round(((balanceData.usedQuota || 0) / balanceData.quota) * 100)
      : 0;
    return [{
      name: selectedPlugin.value?.name || '余额',
      percentage,
      used: balanceData.usedQuota,
      total: balanceData.quota,
      currency: balanceData.currency,
      status: getStatusFromPercentage(percentage, pluginHealthy),
      resetLabel: formatResetLabel(balanceData.expiresAt),
    }];
  }

  return [];
});

// 加载数据（只加载当前选中的插件，而非全部）
const loadData = async (force = false) => {
  console.log('[HomeView] loadData 开始, force=', force);
  try {
    // 1. 获取插件列表和健康状态（轻量级操作）
    const [pluginListResult, allHealthResult] = await Promise.all([
      safeInvoke<Result<PluginInfo[]>>('plugin_list'),
      safeInvoke<Result<PluginHealth[]>>('get_all_health'),
    ]);

    if (pluginListResult.success && pluginListResult.data) {
      plugins.value = pluginListResult.data;
      // 检查当前选中的插件是否仍然可用
      const currentSelection = pluginStore.selectedPluginId;
      const enabledList = plugins.value.filter(p => p.enabled && p.dataType);

      const isCurrentValid = currentSelection && enabledList.some(p => p.id === currentSelection);
      const firstEnabled = enabledList[0];
      if (!isCurrentValid && firstEnabled) {
        // 当前选中的插件不可用，切换到第一个可用插件
        console.log('[HomeView] 当前选中插件不可用，切换到:', firstEnabled.id);
        pluginStore.selectPlugin(firstEnabled.id);
      }
    }

    if (allHealthResult.success && allHealthResult.data) {
      pluginHealth.value = allHealthResult.data;
    }

    // 2. 只刷新当前选中的插件数据
    const targetPluginId = pluginStore.selectedPluginId;
    if (targetPluginId) {
      console.log('[HomeView] 刷新当前插件:', targetPluginId);
      const result = await safeInvoke<Result<PluginData>>('refresh_plugin', { id: targetPluginId, force });
      if (result.success && result.data) {
        // 更新或添加到 pluginData
        const index = pluginData.value.findIndex(d => d.pluginId === targetPluginId);
        if (index >= 0) {
          pluginData.value[index] = result.data;
        } else {
          pluginData.value.push(result.data);
        }
        // 刷新成功，清除该插件的错误
        if (pluginErrors.value.has(targetPluginId)) {
          const newErrors = new Map(pluginErrors.value);
          newErrors.delete(targetPluginId);
          pluginErrors.value = newErrors;
        }
        console.log('[HomeView] 插件数据已更新:', targetPluginId);
      }
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
    // 先从后端获取最新的插件列表（因为 Pinia store 在不同窗口是独立实例）
    await pluginStore.fetchPlugins();
    // 同步到本地状态
    plugins.value = pluginStore.plugins;
    // 然后刷新数据
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

// 管理插件 - 打开主应用的插件页面
const handleManagePlugins = async () => {
  try {
    await safeInvoke('open_dashboard', { route: '/plugins' });
  } catch (e) {
    console.error('打开插件管理失败:', e);
  }
};

// 初始化数据（通过 pluginStore 恢复持久化状态）
const initData = async () => {
  // 使用 pluginStore.init() 恢复持久化的启用状态和配置
  await pluginStore.init();
  // 同步 store 数据到本地状态
  plugins.value = pluginStore.plugins;
  // 获取所有数据
  await loadData();
};

// 监听事件
const setupEventListeners = async () => {
  // 监听插件系统就绪事件（后端初始化完成后发送）
  const unlistenPluginsReady = await safeListen<number>(
    'ipc:plugins_ready',
    async () => {
      console.log('[HomeView] 收到插件就绪事件，重新初始化');
      await initData();
    }
  );
  unlisteners.push(unlistenPluginsReady);

  // 监听托盘刷新事件
  const unlistenTrayRefresh = await safeListen<void>(
    'tray:refresh',
    async () => {
      console.log('[HomeView] 收到托盘刷新事件');
      await handleRefresh();
    }
  );
  unlisteners.push(unlistenTrayRefresh);

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

  // 监听插件安装完成事件（重新加载数据）
  const unlistenPluginInstalled = await safeListen<PluginInfo>(
    'ipc:plugin_installed',
    async (event) => {
      console.log('[HomeView] 收到插件安装事件:', event.payload.id);
      // 重新加载插件列表和数据
      await initData();
    }
  );
  unlisteners.push(unlistenPluginInstalled);

  // 监听插件更新完成事件（重新加载数据）
  const unlistenPluginUpdated = await safeListen<PluginInfo>(
    'ipc:plugin_updated',
    async (event) => {
      console.log('[HomeView] 收到插件更新事件:', event.payload.id);
      // 重新加载插件列表和数据
      await initData();
    }
  );
  unlisteners.push(unlistenPluginUpdated);

  // 监听插件错误事件（插件执行失败时更新 UI）
  const unlistenPluginError = await safeListen<{ id: string; error: { code: string; message: string } }>(
    'ipc:plugin_error',
    (event) => {
      const { id, error: pluginError } = event.payload;
      console.log('[HomeView] 收到插件错误事件:', id, pluginError.message);
      // 更新错误状态
      const newErrors = new Map(pluginErrors.value);
      newErrors.set(id, pluginError);
      pluginErrors.value = newErrors;
    }
  );
  unlisteners.push(unlistenPluginError);

  // 监听窗口焦点变化事件
  if (isTauri) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const currentWindow = getCurrentWindow();
      const unlistenBlur = await currentWindow.onFocusChanged(async ({ payload: focused }) => {
        if (!focused) {
          // 窗口失去焦点时隐藏
          currentWindow.hide();
        } else {
          // 窗口获得焦点时刷新数据（确保与其他窗口同步）
          console.log('[HomeView] 窗口获得焦点，刷新数据');
          await loadData();
        }
      });
      unlisteners.push(unlistenBlur);
    } catch (e) {
      console.warn('设置窗口焦点监听失败:', e);
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
// 插件禁用监听器清理函数
let unlistenPluginDisabled: (() => void) | null = null;
// 插件启用监听器清理函数
let unlistenPluginEnabled: (() => void) | null = null;

// 生命周期
onMounted(async () => {
  console.log('[HomeView] onMounted 开始');

  // 设置透明背景以支持圆角窗口
  document.body.style.background = 'transparent';
  document.documentElement.style.background = 'transparent';

  // 先设置事件监听（避免错过后端早期发送的事件）
  await setupEventListeners();
  console.log('[HomeView] setupEventListeners 完成');

  // 监听跨窗口的插件选择事件（与仪表盘同步）
  unlistenPluginSelection = await pluginStore.setupPluginSelectionListener();

  // 监听插件禁用事件（当其他窗口禁用插件时同步本地状态）
  unlistenPluginDisabled = await pluginStore.setupPluginDisabledListener(async (disabledPluginId) => {
    console.log('[HomeView] 收到插件禁用事件:', disabledPluginId);
    // 从后端重新获取插件列表（因为 Pinia store 在不同窗口是独立实例）
    await pluginStore.fetchPlugins();
    // 同步到本地状态
    plugins.value = pluginStore.plugins;
    // 重新加载数据
    await loadData();
  });

  // 监听插件启用事件（当其他窗口启用插件时同步本地状态）
  unlistenPluginEnabled = await pluginStore.setupPluginEnabledListener(async (enabledPluginId) => {
    console.log('[HomeView] 收到插件启用事件:', enabledPluginId);
    // 从后端重新获取插件列表
    await pluginStore.fetchPlugins();
    // 同步到本地状态
    plugins.value = pluginStore.plugins;
    // 刷新数据
    await loadData();
  });

  // 初始化数据（通过 pluginStore 恢复持久化状态）
  console.log('[HomeView] 开始 initData');
  await initData();
  console.log('[HomeView] initData 完成, plugins=', plugins.value.length, 'pluginData=', pluginData.value.length);
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

  // 清理插件禁用监听
  if (unlistenPluginDisabled) {
    unlistenPluginDisabled();
    unlistenPluginDisabled = null;
  }

  // 清理插件启用监听
  if (unlistenPluginEnabled) {
    unlistenPluginEnabled();
    unlistenPluginEnabled = null;
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
        <!-- 插件错误状态 -->
        <div
          v-if="currentPluginError"
          class="plugin-error-state"
        >
          <div class="plugin-error-icon">⚠️</div>
          <p class="plugin-error-message">{{ currentPluginError.message }}</p>
          <button class="plugin-error-retry" @click="handleRefresh">
            重试
          </button>
        </div>

        <!-- 正常数据展示 -->
        <template v-else>
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
        </template>
      </div>
    </main>

    <!-- 底部栏：系统状态 + 插件图标 + 管理按钮 -->
    <PluginBar
      :plugins="plugins"
      :running-count="enabledDataPlugins.length"
      @manage="handleManagePlugins"
    />
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  height: 100vh; /* 固定高度，确保弹窗布局生效 */
  max-height: 100vh;
  background: var(--color-bg);
  border-radius: 12px;
  overflow: hidden;
}

.home-content {
  flex: 1;
  min-height: 0; /* 关键：确保 flexbox 子元素可以滚动 */
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

/* 插件错误状态 */
.plugin-error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-lg);
  text-align: center;
  background: rgba(239, 68, 68, 0.08);
  border-radius: var(--radius-lg);
  border: 1px solid rgba(239, 68, 68, 0.2);
  margin-top: 12px !important;
}

.plugin-error-icon {
  font-size: 1.5rem;
  margin-bottom: var(--spacing-sm);
}

.plugin-error-message {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-md);
  line-height: 1.4;
  word-break: break-word;
}

.plugin-error-retry {
  background: var(--color-accent-red);
  color: white;
  border: none;
  padding: 6px 16px;
  border-radius: var(--radius-md);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  transition: background var(--transition-fast);
}

.plugin-error-retry:hover {
  background: #dc2626;
}
</style>
