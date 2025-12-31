<script setup lang="ts">
/**
 * 仪表盘视图
 * Phase 8.3: 数据聚合展示、健康状态展示
 */
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import { AppLayout } from '@/components/layout';
import { IconBolt, IconRefresh } from '@/components/icons';
import { usePluginStore } from '@/stores';
import { formatLargeNumber, formatUsedQuota } from '@/utils/format';
import type { UsageData, BalanceData, StatusData, PluginData } from '@/types';

const pluginStore = usePluginStore();
const router = useRouter();

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 安全的事件监听
async function safeListen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void> {
  if (!isTauri) {
    console.info(`[Mock] listen('${event}')`);
    return () => {};
  }
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, handler);
}

// 事件监听器清理函数
const unlisteners: (() => void)[] = [];

// 状态
const isLoading = ref(false);
const isSwitching = ref(false);  // 切换插件时的加载状态
const showPluginDropdown = ref(false);

// 使用 store 中的 selectedPluginId（跨窗口同步）
const selectedPluginId = computed({
  get: () => pluginStore.selectedPluginId,
  set: (val: string) => pluginStore.selectPlugin(val),
});

// 从 Store 获取数据（支持所有数据类型）
const plugins = computed(() => pluginStore.plugins.filter(p => p.enabled && p.dataType));
const hasPlugins = computed(() => plugins.value.length > 0);
const selectedPlugin = computed(() => plugins.value.find(p => p.id === selectedPluginId.value));
// 使用 store 的 computed 确保响应式追踪正确
const healthData = computed(() => pluginStore.selectedPluginHealth);

// 当前数据和类型（使用 store 的 computed 确保响应式追踪正确）
const currentData = computed<PluginData | null>(() => {
  const data = pluginStore.selectedPluginData;
  console.log('[Dashboard] currentData computed:', !!data, data?.dataType);
  return data;
});
const currentDataType = computed(() => currentData.value?.dataType ?? selectedPlugin.value?.dataType);

// 插件下拉框
function toggleDropdown() {
  if (plugins.value.length > 1) {
    showPluginDropdown.value = !showPluginDropdown.value;
  }
}

async function selectPlugin(id: string) {
  showPluginDropdown.value = false;
  // 每次切换都显示加载状态并执行插件刷新
  isSwitching.value = true;
  try {
    await pluginStore.selectPlugin(id);
  } finally {
    isSwitching.value = false;
  }
}

// 获取插件余额显示信息（值和标签分开，用于不同颜色渲染）
function getPluginBalanceInfo(pluginId: string): { value: string; label: string; color: string } {
  const data = pluginStore.pluginData.get(pluginId);
  // 所有数值统一用橙色
  const valueColor = 'var(--color-accent)';
  if (!data) return { value: '--', label: '余额', color: valueColor };

  if (data.dataType === 'balance') {
    const balData = data as BalanceData;
    const formatted = formatBalance(balData.balance, balData.currency);
    return { value: formatted, label: '余额', color: valueColor };
  }
  if (data.dataType === 'usage') {
    const usgData = data as UsageData;
    return { value: `${usgData.percentage}%`, label: '已用', color: valueColor };
  }
  if (data.dataType === 'status') {
    const stsData = data as StatusData;
    const indicator = stsData.indicator ?? 'unknown';
    let statusText = '正常';
    if (indicator === 'minor' || indicator === 'major') {
      statusText = '降级';
    } else if (indicator === 'critical') {
      statusText = '中断';
    }
    return { value: statusText, label: '状态', color: valueColor };
  }
  return { value: '--', label: '余额', color: valueColor };
}

// 跳转到市场
function goToMarketplace() {
  router.push('/marketplace');
}

// 获取使用量数据
const usageData = computed<UsageData | null>(() => {
  const data = currentData.value;
  if (data && data.dataType === 'usage') {
    return data as UsageData;
  }
  return null;
});

// 获取余额数据
const balanceData = computed<BalanceData | null>(() => {
  const data = currentData.value;
  if (data && data.dataType === 'balance') {
    console.log('[Dashboard] balanceData computed: has balance data, items:', (data as BalanceData).items?.length);
    return data as BalanceData;
  }
  console.log('[Dashboard] balanceData computed: null, dataType:', data?.dataType);
  return null;
});

// 获取状态数据
const statusData = computed<StatusData | null>(() => {
  const data = currentData.value;
  if (data && data.dataType === 'status') {
    return data as StatusData;
  }
  return null;
});

// 是否有数据
const hasData = computed(() => {
  const has = currentData.value !== null;
  console.log('[Dashboard] hasData computed:', has);
  return has;
});

// 数据加载状态（智能判断：数据返回或请求完成即停止）
const isDataLoading = computed(() => {
  // 手动刷新时，显示加载状态直到完成
  if (isLoading.value) return true;
  // 如果 store 不在刷新中，说明请求已完成（成功或失败）
  if (!pluginStore.isRefreshing) return false;
  // 如果已有数据，停止加载（数据返回即停止）
  if (hasData.value) return false;
  // 初始化期间且 store 还在刷新
  return isInitializing.value;
});

// 进度条颜色
const progressColor = computed(() => {
  const pct = usageData.value?.percentage ?? 0;
  if (pct >= 90) return 'var(--color-accent-red)';
  if (pct >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
});

// 余额使用百分比颜色
const balanceColor = computed(() => {
  if (!balanceData.value?.quota || !balanceData.value?.usedQuota) return 'var(--color-accent-green)';
  const pct = (balanceData.value.usedQuota / balanceData.value.quota) * 100;
  if (pct >= 90) return 'var(--color-accent-red)';
  if (pct >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
});

// 状态指示器颜色（使用契约定义的 StatusIndicator 类型）
const statusColor = computed(() => {
  const indicator = statusData.value?.indicator ?? 'unknown';
  switch (indicator) {
    case 'none': return 'var(--color-accent-green)';
    case 'minor': return 'var(--color-accent)';
    case 'major': return 'var(--color-accent-red)';
    case 'critical': return 'var(--color-accent-red)';
    default: return 'var(--color-text-tertiary)';
  }
});

// 状态指示器标签
const statusLabel = computed(() => {
  const indicator = statusData.value?.indicator ?? 'unknown';
  switch (indicator) {
    case 'none': return '运行正常';
    case 'minor': return '轻微问题';
    case 'major': return '严重问题';
    case 'critical': return '服务中断';
    default: return '状态未知';
  }
});

// 格式化余额
function formatBalance(balance: number, currency?: string): string {
  if (currency === 'USD') return `$${balance.toFixed(2)}`;
  if (currency === 'CNY') return `¥${balance.toFixed(2)}`;
  return balance.toFixed(2);
}

// 格式化到期时间
function formatExpiresAt(isoTime?: string): string {
  if (!isoTime) return '';
  const expires = new Date(isoTime);
  const now = new Date();
  const diff = expires.getTime() - now.getTime();
  if (diff <= 0) return '已过期';
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days > 30) return `${Math.floor(days / 30)}个月后到期`;
  if (days > 0) return `${days}天后到期`;
  const hours = Math.floor(diff / (1000 * 60 * 60));
  return `${hours}小时后到期`;
}

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

// ============================================================================
// Balance Items 辅助函数
// ============================================================================

// 获取 item 进度条颜色
function getItemProgressColor(percentage: number): string {
  if (percentage >= 90) return 'var(--color-accent-red)';
  if (percentage >= 75) return 'var(--color-accent)';
  return 'var(--color-accent-green)';
}

// 格式化 item 已用量显示（显示"已用 X/Y"）
function formatItemUsed(item: { used: number; quota: number; percentage: number; currency?: string }): string {
  // 按量付费模式（quota 为 0）显示已用金额
  if (!item.quota || item.quota === 0) {
    const currency = item.currency || '$';
    return `已用 ${formatLargeNumber(item.used)} ${currency}`;
  }
  // 有配额时显示"已用 X/Y"
  return formatUsedQuota(item.used, item.quota);
}

// 格式化 item 重置/到期信息
function formatItemReset(item: {
  resetLabel?: string;
  resetTime?: string;
  expiresAt?: string;
  remainingDays?: number;
}): string {
  // 优先使用 resetLabel
  if (item.resetLabel) return item.resetLabel;

  // 显示剩余天数
  if (item.remainingDays !== undefined) {
    if (item.remainingDays <= 0) return '已到期';
    return `剩余 ${item.remainingDays} 天`;
  }

  // 格式化重置时间
  if (item.resetTime) {
    return formatResetTime(item.resetTime);
  }

  // 格式化到期时间
  if (item.expiresAt) {
    return formatExpiresAt(item.expiresAt);
  }

  return '';
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

// 跳转到插件配置
function goToPluginConfig() {
  if (selectedPluginId.value) {
    router.push({ path: '/settings', query: { plugin: selectedPluginId.value } });
  }
}

// 初始化状态
const isInitializing = ref(true);

// 设置事件监听
async function setupEventListeners(): Promise<void> {
  // 监听插件系统就绪事件（后端初始化完成后发送）
  const unlistenPluginsReady = await safeListen<number>(
    'ipc:plugins_ready',
    async () => {
      console.log('[DashboardView] 收到插件就绪事件，重新初始化');
      await pluginStore.init();
      if (pluginStore.selectedPluginId) {
        await pluginStore.refreshPlugin(pluginStore.selectedPluginId, true);
      }
    }
  );
  unlisteners.push(unlistenPluginsReady);

  // 监听托盘刷新事件
  const unlistenTrayRefresh = await safeListen<void>(
    'tray:refresh',
    async () => {
      console.log('[DashboardView] 收到托盘刷新事件');
      await refreshData();
    }
  );
  unlisteners.push(unlistenTrayRefresh);

  // 监听插件禁用事件（其他窗口禁用插件时同步状态）
  const unlistenPluginDisabled = await pluginStore.setupPluginDisabledListener(async (disabledPluginId) => {
    console.log('[DashboardView] 收到插件禁用事件:', disabledPluginId);
    // 关闭下拉菜单（如果正在显示）
    showPluginDropdown.value = false;
    // 从后端重新获取插件列表（因为 Pinia store 在不同窗口是独立实例）
    await pluginStore.fetchPlugins();
    await pluginStore.fetchAllData();
  });
  unlisteners.push(unlistenPluginDisabled);

  // 监听插件启用事件（其他窗口启用插件时同步状态）
  const unlistenPluginEnabled = await pluginStore.setupPluginEnabledListener(async (enabledPluginId) => {
    console.log('[DashboardView] 收到插件启用事件:', enabledPluginId);
    // 从后端重新获取插件列表和数据
    await pluginStore.fetchPlugins();
    await pluginStore.fetchAllData();
  });
  unlisteners.push(unlistenPluginEnabled);
}

// 初始化（带超时保护）
onMounted(async () => {
  // 先设置事件监听（避免错过后端早期发送的事件）
  await setupEventListeners();

  isInitializing.value = true;

  // 超时保护：最多等待 10 秒
  const timeout = new Promise<void>((_, reject) =>
    setTimeout(() => reject(new Error('初始化超时')), 10000)
  );

  try {
    await Promise.race([
      (async () => {
        // init 会恢复持久化的选择并加载所有插件数据
        await pluginStore.init();
        // 如果 store 中没有选中插件，选择第一个有数据类型的插件
        if (!pluginStore.selectedPluginId) {
          const firstPlugin = plugins.value[0];
          if (firstPlugin) {
            await pluginStore.selectPlugin(firstPlugin.id);
          }
        }
        // init() 已经调用了 fetchAllData()，无需再次刷新
        // 但如果当前插件数据缺失，单独刷新一次
        if (pluginStore.selectedPluginId && !pluginStore.pluginData.has(pluginStore.selectedPluginId)) {
          console.log('[DashboardView] 当前插件数据缺失，单独刷新:', pluginStore.selectedPluginId);
          await pluginStore.refreshPlugin(pluginStore.selectedPluginId, true);
        }
      })(),
      timeout,
    ]);
  } catch (e) {
    console.error('[DashboardView] 初始化失败:', e);
  } finally {
    isInitializing.value = false;
  }
  // 若无插件，selectedPluginId 保持空，UI 会显示空状态
});

onUnmounted(() => {
  // 清理事件监听器
  unlisteners.forEach(unlisten => unlisten());
});
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>仪表盘</h2>
    </template>

    <div class="dashboard">
      <!-- 空状态（初始化完成且无插件时显示） -->
      <div v-if="!isInitializing && !hasPlugins" class="empty-state">
        <!-- 背景装饰 -->
        <div class="empty-bg-decoration">
          <div class="decoration-circle decoration-circle-1"></div>
          <div class="decoration-circle decoration-circle-2"></div>
          <div class="decoration-circle decoration-circle-3"></div>
        </div>

        <!-- 精美的 SVG 插画 -->
        <div class="empty-illustration">
          <svg width="180" height="140" viewBox="0 0 180 140" fill="none" xmlns="http://www.w3.org/2000/svg">
            <!-- 底部平台 -->
            <ellipse cx="90" cy="130" rx="70" ry="8" fill="url(#platformGradient)" opacity="0.3"/>

            <!-- 主仪表盘面板 -->
            <rect x="35" y="30" width="110" height="85" rx="12" fill="url(#panelGradient)" stroke="url(#borderGradient)" stroke-width="1.5"/>

            <!-- 面板内部装饰线 -->
            <line x1="50" y1="50" x2="130" y2="50" stroke="var(--color-border)" stroke-width="1" opacity="0.5"/>

            <!-- 柱状图 -->
            <rect x="55" y="75" width="16" height="30" rx="3" fill="url(#barGradient1)" class="bar bar-1"/>
            <rect x="82" y="60" width="16" height="45" rx="3" fill="url(#barGradient2)" class="bar bar-2"/>
            <rect x="109" y="68" width="16" height="37" rx="3" fill="url(#barGradient3)" class="bar bar-3"/>

            <!-- 仪表盘圆环（右上角装饰） -->
            <circle cx="145" cy="25" r="20" fill="var(--color-bg-tertiary)" stroke="url(#ringGradient)" stroke-width="3" opacity="0.8"/>
            <path d="M145 13 A12 12 0 1 1 133 25" stroke="var(--color-accent)" stroke-width="3" stroke-linecap="round" class="gauge-arc"/>

            <!-- 浮动数据点 -->
            <circle cx="30" cy="50" r="6" fill="var(--color-accent-green)" class="float-dot dot-1"/>
            <circle cx="155" cy="70" r="4" fill="var(--color-accent)" class="float-dot dot-2"/>
            <circle cx="25" cy="85" r="3" fill="var(--color-accent-red)" class="float-dot dot-3"/>

            <!-- 连接线 -->
            <path d="M36 50 L55 75" stroke="var(--color-accent-green)" stroke-width="1" stroke-dasharray="3 3" opacity="0.5"/>
            <path d="M151 70 L125 68" stroke="var(--color-accent)" stroke-width="1" stroke-dasharray="3 3" opacity="0.5"/>

            <!-- 渐变定义 -->
            <defs>
              <linearGradient id="platformGradient" x1="20" y1="130" x2="160" y2="130">
                <stop offset="0%" stop-color="var(--color-text-tertiary)" stop-opacity="0"/>
                <stop offset="50%" stop-color="var(--color-text-tertiary)"/>
                <stop offset="100%" stop-color="var(--color-text-tertiary)" stop-opacity="0"/>
              </linearGradient>
              <linearGradient id="panelGradient" x1="35" y1="30" x2="145" y2="115">
                <stop offset="0%" stop-color="var(--color-bg-tertiary)"/>
                <stop offset="100%" stop-color="var(--color-bg-secondary)"/>
              </linearGradient>
              <linearGradient id="borderGradient" x1="35" y1="30" x2="145" y2="115">
                <stop offset="0%" stop-color="var(--color-border)" stop-opacity="0.8"/>
                <stop offset="100%" stop-color="var(--color-border)" stop-opacity="0.3"/>
              </linearGradient>
              <linearGradient id="barGradient1" x1="63" y1="105" x2="63" y2="75">
                <stop offset="0%" stop-color="var(--color-accent-green)"/>
                <stop offset="100%" stop-color="var(--color-accent-green)" stop-opacity="0.6"/>
              </linearGradient>
              <linearGradient id="barGradient2" x1="90" y1="105" x2="90" y2="60">
                <stop offset="0%" stop-color="var(--color-accent)"/>
                <stop offset="100%" stop-color="var(--color-accent)" stop-opacity="0.6"/>
              </linearGradient>
              <linearGradient id="barGradient3" x1="117" y1="105" x2="117" y2="68">
                <stop offset="0%" stop-color="#6366f1"/>
                <stop offset="100%" stop-color="#6366f1" stop-opacity="0.6"/>
              </linearGradient>
              <linearGradient id="ringGradient" x1="125" y1="5" x2="165" y2="45">
                <stop offset="0%" stop-color="var(--color-border)"/>
                <stop offset="100%" stop-color="var(--color-border)" stop-opacity="0.3"/>
              </linearGradient>
            </defs>
          </svg>
        </div>

        <!-- 文字内容 -->
        <div class="empty-content">
          <h3>暂无用量监控插件</h3>
          <p>安装插件后即可在此查看 AI 服务的使用量、余额等数据</p>
        </div>

        <!-- 功能特性提示 -->
        <div class="empty-features">
          <div class="feature-item">
            <div class="feature-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M22 12h-4l-3 9L9 3l-3 9H2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <span>实时监控</span>
          </div>
          <div class="feature-item">
            <div class="feature-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <rect x="3" y="3" width="7" height="7" rx="1" stroke="currentColor" stroke-width="2"/>
                <rect x="14" y="3" width="7" height="7" rx="1" stroke="currentColor" stroke-width="2"/>
                <rect x="14" y="14" width="7" height="7" rx="1" stroke="currentColor" stroke-width="2"/>
                <rect x="3" y="14" width="7" height="7" rx="1" stroke="currentColor" stroke-width="2"/>
              </svg>
            </div>
            <span>多源聚合</span>
          </div>
          <div class="feature-item">
            <div class="feature-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <span>安全可靠</span>
          </div>
        </div>

        <!-- 行动按钮 -->
        <button class="go-marketplace-btn" @click="goToMarketplace">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
            <path d="M6 2L3 6v14a2 2 0 002 2h14a2 2 0 002-2V6l-3-4z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            <line x1="3" y1="6" x2="21" y2="6" stroke="currentColor" stroke-width="2"/>
            <path d="M16 10a4 4 0 01-8 0" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span>前往插件市场</span>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" class="arrow-icon">
            <path d="M5 12h14M12 5l7 7-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>

      <!-- 主插件卡片（初始化中或有插件时显示） -->
      <div v-else class="plugin-card">
        <!-- 切换加载蒙层 -->
        <div v-if="isSwitching" class="switching-overlay">
          <div class="switching-spinner"></div>
          <span class="switching-text">加载中...</span>
        </div>

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
                <span class="plugin-name">{{ selectedPlugin?.name || '加载中...' }}</span>
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
                <div class="dropdown-label">切换监控源</div>
                <div
                  v-for="plugin in plugins"
                  :key="plugin.id"
                  class="dropdown-item"
                  :class="{ active: plugin.id === selectedPluginId }"
                  @click="selectPlugin(plugin.id)"
                >
                  <div class="dropdown-item-icon">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                      <rect x="3" y="4" width="18" height="14" rx="2" stroke="currentColor" stroke-width="2"/>
                      <line x1="7" y1="11" x2="17" y2="11" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                  </div>
                  <div class="dropdown-item-content">
                    <span class="dropdown-item-name">{{ plugin.name }}</span>
                    <span class="dropdown-item-balance"><span class="balance-value" :style="{ color: getPluginBalanceInfo(plugin.id).color }">{{ getPluginBalanceInfo(plugin.id).value }}</span><span class="balance-dot">·</span><span class="balance-label">{{ getPluginBalanceInfo(plugin.id).label }}</span></span>
                  </div>
                  <svg
                    v-if="plugin.id === selectedPluginId"
                    class="dropdown-check"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                  >
                    <path
                      d="M5 12l5 5L20 7"
                      stroke="currentColor"
                      stroke-width="3"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
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

        <!-- 数据展示区域（支持局部刷新） -->
        <div class="data-section" :class="{ refreshing: isDataLoading }">
          <!-- 局部刷新蒙层（有数据后自动隐藏） -->
          <div v-if="isDataLoading" class="refresh-overlay">
            <div class="refresh-spinner"></div>
          </div>

          <!-- 无数据状态（仅在初始化完成后显示） -->
          <div v-if="!isInitializing && !hasData" class="no-data-state">
            <div class="no-data-icon">⚙️</div>
            <h4>需要配置插件</h4>
            <p>请先配置插件的 API 密钥等参数</p>
            <button class="config-btn" @click="goToPluginConfig">前往配置</button>
          </div>

          <!-- Usage 类型展示 -->
          <template v-else-if="currentDataType === 'usage' && usageData">
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
                {{ usageData.resetLabel || '--' }}
              </div>
              <div class="usage-detail">
                {{ formatUsedQuota(usageData.used ?? 0, usageData.limit ?? 0) }} {{ usageData.unit }}
              </div>
            </div>
          </div>

          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{ width: usageData.percentage + '%', background: progressColor }"
            />
          </div>

          <div v-if="usageData.dimensions?.length" class="dimensions-section">
            <div class="section-header">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                <polyline points="14,2 14,8 20,8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
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
                  <div class="dimension-progress-fill" :style="{ width: dim.percentage + '%', background: dim.percentage >= 75 ? 'var(--color-accent)' : 'var(--color-accent-green)' }" />
                </div>
                <div class="dimension-meta">
                  <span>{{ formatLargeNumber(dim.used ?? 0) }}/{{ formatLargeNumber(dim.limit ?? 0) }}</span>
                  <span>{{ formatResetTime(dim.resetTime) }}</span>
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- Balance 类型展示 -->
        <template v-else-if="currentDataType === 'balance' && balanceData">
          <!-- 主余额显示（仅当 showTotal=true 且有 balance 值时） -->
          <div v-if="balanceData.showTotal && balanceData.balance > 0" class="balance-main">
            <div class="balance-stats">
              <span class="balance-label">当前余额</span>
              <div class="balance-value">
                <span class="balance-amount accent">{{ formatBalance(balanceData.balance, balanceData.currency) }}</span>
              </div>
            </div>
            <div class="balance-meta">
              <div v-if="balanceData.expiresAt" class="expires-badge">
                {{ formatExpiresAt(balanceData.expiresAt) }}
              </div>
            </div>
          </div>

          <!-- 多配额子项网格 -->
          <div v-if="balanceData.items?.length" class="items-grid">
            <div v-for="(item, index) in balanceData.items" :key="`${item.name}-${index}`" class="item-card">
              <div class="item-header">
                <span class="item-name">{{ item.name }}</span>
                <span v-if="item.refreshable" class="refreshable-badge">可刷新</span>
                <!-- PAY_PER_USE (quota=0) 不显示百分比；显示剩余百分比 -->
                <span v-if="item.quota > 0" class="item-percentage">{{ Math.round(100 - item.percentage) }}%</span>
              </div>
              <!-- PAY_PER_USE (quota=0) 不显示进度条 -->
              <div v-if="item.quota > 0" class="item-progress">
                <div
                  class="item-progress-fill"
                  :style="{
                    width: item.percentage + '%',
                    background: getItemProgressColor(item.percentage)
                  }"
                />
              </div>
              <div class="item-meta">
                <span class="item-used">{{ formatItemUsed(item) }}</span>
                <span class="item-reset">{{ formatItemReset(item) }}</span>
              </div>
            </div>
          </div>

          <!-- 旧版额度使用（无 items 时的 fallback） -->
          <div v-else-if="balanceData.quota && balanceData.usedQuota !== undefined" class="quota-section">
            <div class="section-header">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" />
                <path d="M12 6v6l4 2" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              </svg>
              <span>额度使用</span>
            </div>
            <div class="quota-info">
              <div class="quota-used">
                {{ formatUsedQuota(balanceData.usedQuota ?? 0, balanceData.quota ?? 0) }} {{ balanceData.currency }}
              </div>
              <div class="quota-progress">
                <div class="quota-progress-fill" :style="{ width: (balanceData.usedQuota / balanceData.quota * 100) + '%', background: balanceColor }" />
              </div>
            </div>
          </div>
        </template>

        <!-- Status 类型展示 -->
        <template v-else-if="currentDataType === 'status' && statusData">
          <div class="status-main">
            <div class="status-indicator-large" :style="{ background: statusColor }">
              <span class="status-icon">{{ statusData.indicator === 'none' ? '✓' : '!' }}</span>
            </div>
            <div class="status-info">
              <span class="status-title">{{ statusLabel }}</span>
              <p v-if="statusData.description" class="status-description">{{ statusData.description }}</p>
            </div>
          </div>
        </template>
        </div><!-- /.data-section -->

        <!-- 连接监控 -->
        <div class="monitoring-section">
          <div class="section-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <rect x="2" y="3" width="20" height="14" rx="2" stroke="currentColor" stroke-width="2" />
              <line x1="8" y1="21" x2="16" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              <line x1="12" y1="17" x2="12" y2="21" stroke="currentColor" stroke-width="2" />
            </svg>
            <span>连接监控 (RELIABILITY LAYER)</span>
          </div>
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
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-3xl) var(--spacing-xl);
  text-align: center;
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  overflow: hidden;
  min-height: 480px;
}

/* 背景装饰圆圈 */
.empty-bg-decoration {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.decoration-circle {
  position: absolute;
  border-radius: 50%;
  opacity: 0.08;
}

.decoration-circle-1 {
  width: 300px;
  height: 300px;
  background: var(--color-accent);
  top: -100px;
  right: -80px;
  animation: floatSlow 20s ease-in-out infinite;
}

.decoration-circle-2 {
  width: 200px;
  height: 200px;
  background: var(--color-accent-green);
  bottom: -60px;
  left: -40px;
  animation: floatSlow 15s ease-in-out infinite reverse;
}

.decoration-circle-3 {
  width: 120px;
  height: 120px;
  background: #6366f1;
  top: 50%;
  left: 10%;
  animation: floatSlow 18s ease-in-out infinite 2s;
}

@keyframes floatSlow {
  0%, 100% { transform: translate(0, 0) scale(1); }
  25% { transform: translate(10px, -15px) scale(1.02); }
  50% { transform: translate(-5px, 10px) scale(0.98); }
  75% { transform: translate(-10px, -5px) scale(1.01); }
}

/* 插画容器 */
.empty-illustration {
  position: relative;
  z-index: 1;
  margin-bottom: var(--spacing-xl);
  animation: fadeInUp 0.6s ease-out;
}

.empty-illustration svg {
  filter: drop-shadow(0 8px 24px rgba(0, 0, 0, 0.15));
}

/* 柱状图动画 */
.empty-illustration .bar {
  transform-origin: bottom;
  animation: barGrow 1s ease-out forwards;
}

.empty-illustration .bar-1 { animation-delay: 0.2s; }
.empty-illustration .bar-2 { animation-delay: 0.4s; }
.empty-illustration .bar-3 { animation-delay: 0.6s; }

@keyframes barGrow {
  0% { transform: scaleY(0); opacity: 0; }
  100% { transform: scaleY(1); opacity: 1; }
}

/* 浮动点动画 */
.empty-illustration .float-dot {
  animation: floatDot 3s ease-in-out infinite;
}

.empty-illustration .dot-1 { animation-delay: 0s; }
.empty-illustration .dot-2 { animation-delay: 1s; }
.empty-illustration .dot-3 { animation-delay: 2s; }

@keyframes floatDot {
  0%, 100% { transform: translateY(0); opacity: 0.8; }
  50% { transform: translateY(-8px); opacity: 1; }
}

/* 仪表盘圆弧动画 */
.empty-illustration .gauge-arc {
  stroke-dasharray: 60;
  stroke-dashoffset: 60;
  animation: gaugeArc 1.5s ease-out 0.5s forwards;
}

@keyframes gaugeArc {
  to { stroke-dashoffset: 0; }
}

/* 文字内容 */
.empty-content {
  position: relative;
  z-index: 1;
  animation: fadeInUp 0.6s ease-out 0.2s backwards;
}

.empty-content h3 {
  font-size: 1.375rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: var(--spacing-sm);
  letter-spacing: -0.01em;
}

.empty-content p {
  font-size: 0.9375rem;
  color: var(--color-text-secondary);
  max-width: 320px;
  line-height: 1.6;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 功能特性 */
.empty-features {
  display: flex;
  gap: var(--spacing-xl);
  margin: var(--spacing-xl) 0;
  position: relative;
  z-index: 1;
  animation: fadeInUp 0.6s ease-out 0.4s backwards;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  color: var(--color-text-tertiary);
  font-size: 0.8125rem;
  transition: color var(--transition-fast);
}

.feature-item:hover {
  color: var(--color-text-secondary);
}

.feature-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast);
}

.feature-item:hover .feature-icon {
  background: var(--color-bg-hover);
  color: var(--color-accent);
}

/* 行动按钮 */
.go-marketplace-btn {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  background: linear-gradient(135deg, var(--color-accent) 0%, #e67e00 100%);
  color: white;
  border: none;
  padding: var(--spacing-md) var(--spacing-xl);
  border-radius: var(--radius-lg);
  font-size: 0.9375rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-normal);
  box-shadow: 0 4px 14px rgba(249, 115, 22, 0.35);
  animation: fadeInUp 0.6s ease-out 0.6s backwards;
}

.go-marketplace-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(249, 115, 22, 0.45);
}

.go-marketplace-btn:active {
  transform: translateY(0);
}

.go-marketplace-btn .arrow-icon {
  transition: transform var(--transition-fast);
}

.go-marketplace-btn:hover .arrow-icon {
  transform: translateX(4px);
}

.plugin-card {
  position: relative;
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  padding: var(--spacing-xl);
}

/* 切换加载蒙层 */
.switching-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-md);
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  border-radius: var(--radius-xl);
  z-index: 50;
}

.switching-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--color-border);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.switching-text {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
}

/* 数据展示区域（局部刷新） */
.data-section {
  position: relative;
  min-height: 120px;
  transition: opacity var(--transition-fast);
}

.data-section.refreshing {
  pointer-events: none;
}

.refresh-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(var(--color-bg-card-rgb, 30, 30, 30), 0.7);
  backdrop-filter: blur(2px);
  border-radius: var(--radius-lg);
  z-index: 10;
}

.refresh-spinner {
  width: 28px;
  height: 28px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
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
  left: -60px;
  min-width: 280px;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  z-index: 100;
  margin-top: var(--spacing-sm);
  overflow: hidden;
  border: 1px solid var(--color-border);
}

.dropdown-label {
  font-size: 0.8125rem;
  color: var(--color-text-tertiary);
  padding: 10px 16px;
  background: var(--color-bg-tertiary);
}

.dropdown-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  cursor: pointer;
  transition: background var(--transition-fast);
  border-top: 1px solid var(--color-border);
}

.dropdown-item:hover {
  background: var(--color-bg-hover);
}

.dropdown-item-icon {
  width: 36px;
  height: 36px;
  background: rgba(34, 197, 94, 0.15);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-accent-green);
  flex-shrink: 0;
}

.dropdown-item-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.dropdown-item-name {
  font-size: 1rem;
  font-weight: 500;
  color: var(--color-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dropdown-item-balance {
  font-size: 0.875rem;
  line-height: 1.2;
  white-space: nowrap;
  display: inline;
}

.dropdown-item-balance .balance-value,
.dropdown-item-balance .balance-dot,
.dropdown-item-balance .balance-label {
  display: inline;
}

.dropdown-item-balance .balance-value {
  font-weight: 500;
}

.dropdown-item-balance .balance-dot,
.dropdown-item-balance .balance-label {
  color: var(--color-text-tertiary);
}

.dropdown-check {
  color: var(--color-accent);
  flex-shrink: 0;
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

/* 无数据状态 */
.no-data-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-xl);
  text-align: center;
}

.no-data-icon {
  font-size: 3rem;
  margin-bottom: var(--spacing-md);
}

.no-data-state h4 {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: var(--spacing-xs);
}

.no-data-state p {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-lg);
}

.config-btn {
  background: var(--color-accent);
  color: white;
  border: none;
  padding: var(--spacing-sm) var(--spacing-lg);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.config-btn:hover {
  background: var(--color-accent-hover);
}

/* Balance 类型样式 */
.balance-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: var(--spacing-xl);
}

.balance-stats {
  display: flex;
  flex-direction: column;
}

.balance-label {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-sm);
}

.balance-value {
  display: flex;
  align-items: baseline;
}

.balance-amount {
  font-size: 3rem;
  font-weight: 700;
  color: var(--color-text);
  line-height: 1;
}

.balance-amount.accent {
  color: var(--color-accent);
}

.balance-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.expires-badge {
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
}

.quota-section {
  margin-top: var(--spacing-xl);
  padding-top: var(--spacing-xl);
  border-top: 1px solid var(--color-border);
}

.quota-info {
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
}

.quota-used {
  font-size: 0.875rem;
  color: var(--color-text);
  margin-bottom: var(--spacing-sm);
}

.quota-progress {
  height: 8px;
  background: var(--color-bg-secondary);
  border-radius: 4px;
  overflow: hidden;
}

.quota-progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width var(--transition-normal);
}

/* Balance Items 卡片网格 */
.items-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-md);
}

.item-card {
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
}

.item-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
}

.item-name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
}

.refreshable-badge {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
  padding: 2px var(--spacing-sm);
  border-radius: var(--radius-sm);
  font-size: 0.6875rem;
  font-weight: 500;
  white-space: nowrap;
  flex-shrink: 0;
}

.item-percentage {
  margin-left: auto;
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
}

.item-progress {
  height: 6px;
  background: var(--color-bg-secondary);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: var(--spacing-sm);
}

.item-progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width var(--transition-normal);
}

.item-meta {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.item-reset {
  color: var(--color-accent);
}

/* Status 类型样式 */
.status-main {
  display: flex;
  align-items: center;
  gap: var(--spacing-xl);
  padding: var(--spacing-xl) 0;
}

.status-indicator-large {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.status-icon {
  font-size: 2rem;
  color: white;
}

.status-info {
  flex: 1;
}

.status-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--color-text);
  display: block;
  margin-bottom: var(--spacing-sm);
}

.status-description {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin: 0;
  line-height: 1.5;
}
</style>
