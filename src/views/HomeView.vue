<script setup lang="ts">
/**
 * Phase 8.1: 托盘弹窗主视图
 * 集成所有卡片组件，显示插件数据和状态
 */
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { TrayHeader, UsageCard, BalanceCard, StatusCard, PluginBar } from '@/components/tray';
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
      invoke<Result<PluginInfo[]>>('plugin_list'),
      invoke<Result<PluginData[]>>('get_all_data'),
      invoke<Result<PluginHealth[]>>('get_all_health'),
      invoke<string>('get_version').catch(() => '2.2'),
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
    const result = await invoke<Result<PluginData[]>>('refresh_all', { force: true });
    if (result.success && result.data) {
      pluginData.value = result.data;
    }
    // 重新加载健康状态
    const healthResult = await invoke<Result<PluginHealth[]>>('get_all_health');
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
    // 通过 Tauri 事件请求打开设置窗口
    const currentWindow = getCurrentWindow();
    await currentWindow.emit('open-settings');
  } catch (e) {
    console.error('打开设置失败:', e);
  }
};

// 管理插件
const handleManagePlugins = async () => {
  try {
    const currentWindow = getCurrentWindow();
    await currentWindow.emit('open-settings', { tab: 'plugins' });
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
  const unlistenDataUpdated = await listen<{ id: string; data: PluginData }>(
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
  const unlistenHealthChanged = await listen<PluginHealth>(
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
      <div v-if="error" class="error-banner">
        <span>{{ error }}</span>
        <button @click="loadData">重试</button>
      </div>

      <!-- 使用量卡片 -->
      <UsageCard
        v-for="usage in usageDataList"
        :key="usage.pluginId"
        :data="usage"
        :plugin-name="getPluginName(usage.pluginId)"
        :plugin-id="usage.pluginId"
      />

      <!-- 空状态: 没有使用量数据时显示示例 -->
      <UsageCard
        v-if="usageDataList.length === 0"
        :data="{
          pluginId: 'demo-usage',
          dataType: 'usage',
          percentage: 78,
          used: 780,
          limit: 1000,
          unit: 'requests',
          resetTime: new Date(Date.now() + 2 * 60 * 60 * 1000 + 15 * 60 * 1000).toISOString(),
          lastUpdated: new Date().toISOString(),
        }"
        plugin-name="Claude Usage"
        plugin-id="claude-usage"
      />

      <!-- 余额卡片组 -->
      <div v-if="balanceDataList.length > 0" class="balance-row">
        <BalanceCard
          v-for="(balance, index) in balanceDataList"
          :key="balance.pluginId"
          :data="balance"
          :plugin-name="getPluginName(balance.pluginId)"
          :health-status="getPluginHealthStatus(balance.pluginId)"
          :color-theme="['green', 'blue', 'orange', 'purple'][index % 4] as 'green' | 'blue' | 'orange' | 'purple'"
        />
      </div>

      <!-- 空状态: 没有余额数据时显示示例 -->
      <div v-else class="balance-row">
        <BalanceCard
          :data="{
            pluginId: 'demo-openai',
            dataType: 'balance',
            balance: 12.45,
            currency: 'USD',
            lastUpdated: new Date().toISOString(),
          }"
          plugin-name="OpenAI API"
          health-status="healthy"
          color-theme="green"
        />
        <BalanceCard
          :data="{
            pluginId: 'demo-deepseek',
            dataType: 'balance',
            balance: 45,
            currency: 'CNY',
            lastUpdated: new Date().toISOString(),
          }"
          plugin-name="DeepSeek"
          health-status="healthy"
          color-theme="blue"
        />
      </div>

      <!-- 系统状态卡片 -->
      <StatusCard
        :indicator="systemStatusIndicator"
        description="System Status"
        :monitored-count="plugins.length"
        expandable
      />
    </main>

    <!-- 底部插件栏 -->
    <PluginBar
      :plugins="plugins.length > 0 ? plugins : [
        { id: 'claude', name: 'Claude', version: '1.0', pluginType: 'data', enabled: true, healthy: true },
        { id: 'openai', name: 'OpenAI', version: '1.0', pluginType: 'data', enabled: true, healthy: true },
        { id: 'deepseek', name: 'DeepSeek', version: '1.0', pluginType: 'data', enabled: true, healthy: true },
      ]"
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
</style>
