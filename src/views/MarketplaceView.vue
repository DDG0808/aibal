<script setup lang="ts">
/**
 * 插件市场视图
 * 从远程仓库获取插件列表，支持搜索和安装
 */
import { ref, watch, onMounted, onUnmounted, computed } from 'vue';
import { AppLayout } from '@/components/layout';
import { IconSearch, IconDownload, IconVerified, IconRefresh } from '@/components/icons';
import { usePluginStore } from '@/stores';
import { marketplaceService } from '@/services';
import type { MarketplacePlugin, InstallStatus } from '@/types';

const pluginStore = usePluginStore();

// debounce timer
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function debounceFn<T extends (...args: unknown[]) => void>(fn: T, delay: number): T {
  return ((...args: unknown[]) => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => fn(...args), delay);
  }) as T;
}

onUnmounted(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
});

// ============================================================================
// 加载状态
// ============================================================================

const isLoadingPlugins = ref(true);
const isRefreshing = ref(false);
const loadError = ref<string | null>(null);
const lastRefreshTime = ref<Date | null>(null);

// ============================================================================
// 搜索
// ============================================================================

const searchQuery = ref('');
const isSearching = ref(false);
const searchResults = ref<MarketplacePlugin[]>([]);

// ============================================================================
// 插件列表
// ============================================================================

const allPlugins = ref<MarketplacePlugin[]>([]);

// ============================================================================
// 初始化加载
// ============================================================================

onMounted(async () => {
  await loadPlugins();
});

async function loadPlugins() {
  try {
    isLoadingPlugins.value = true;
    loadError.value = null;
    allPlugins.value = await marketplaceService.getAllPlugins();
    lastRefreshTime.value = new Date();

    // 检查是否有错误
    const error = marketplaceService.getLastError();
    if (error && allPlugins.value.length === 0) {
      loadError.value = error;
    }
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : '加载失败';
    console.error('Failed to load plugins:', e);
  } finally {
    isLoadingPlugins.value = false;
  }
}

// ============================================================================
// 刷新功能
// ============================================================================

async function refreshPlugins() {
  if (isRefreshing.value) return;

  try {
    isRefreshing.value = true;
    loadError.value = null;

    const result = await marketplaceService.refreshRegistry();

    if (result.success) {
      allPlugins.value = await marketplaceService.getAllPlugins();
      lastRefreshTime.value = new Date();
    } else {
      loadError.value = result.error || '刷新失败';
    }
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : '刷新失败';
  } finally {
    isRefreshing.value = false;
  }
}

// 格式化刷新时间
const formattedRefreshTime = computed(() => {
  if (!lastRefreshTime.value) return '';
  const now = new Date();
  const diff = Math.floor((now.getTime() - lastRefreshTime.value.getTime()) / 1000);

  if (diff < 60) return '刚刚更新';
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前更新`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前更新`;
  return lastRefreshTime.value.toLocaleDateString();
});

// ============================================================================
// 搜索功能
// ============================================================================

async function performSearch() {
  if (!searchQuery.value.trim()) {
    searchResults.value = [];
    isSearching.value = false;
    return;
  }
  try {
    searchResults.value = await marketplaceService.searchPlugins(searchQuery.value);
  } catch (e) {
    console.error('Search failed:', e);
    searchResults.value = [];
  } finally {
    isSearching.value = false;
  }
}

const debouncedSearch = debounceFn(performSearch, 300);

watch(searchQuery, (newVal) => {
  if (!newVal.trim()) {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    searchResults.value = [];
    isSearching.value = false;
    return;
  }
  isSearching.value = true;
  debouncedSearch();
});

// ============================================================================
// 安装状态
// ============================================================================

function getPluginStatus(pluginId: string): InstallStatus | 'installed' | 'confirm' {
  if (pluginStore.isInstalled(pluginId)) {
    return 'installed';
  }
  if (pluginStore.needsSignatureConfirm(pluginId)) {
    return 'confirm';
  }
  return pluginStore.getInstallStatus(pluginId);
}

function getButtonText(pluginId: string): string {
  const status = getPluginStatus(pluginId);
  switch (status) {
    case 'installed':
      return '已安装';
    case 'downloading':
      return '下载中...';
    case 'installing':
      return '安装中...';
    case 'success':
      return '完成';
    case 'error':
      return '重试';
    case 'confirm':
      return '确认安装';
    default:
      return '';
  }
}

function isButtonDisabled(pluginId: string): boolean {
  const status = getPluginStatus(pluginId);
  return status === 'installed' || status === 'downloading' || status === 'installing' || status === 'success';
}

function shouldShowIcon(pluginId: string): boolean {
  const status = getPluginStatus(pluginId);
  return status === 'idle' || status === 'error';
}

// ============================================================================
// 签名确认对话框
// ============================================================================

const showSignatureDialog = ref(false);
const confirmingPluginId = ref<string | null>(null);
const confirmingPluginName = ref<string>('');

function showSignatureConfirmDialog(pluginId: string, pluginName: string) {
  confirmingPluginId.value = pluginId;
  confirmingPluginName.value = pluginName;
  showSignatureDialog.value = true;
}

function closeSignatureDialog() {
  if (confirmingPluginId.value) {
    pluginStore.cancelSignatureConfirm(confirmingPluginId.value);
  }
  showSignatureDialog.value = false;
  confirmingPluginId.value = null;
  confirmingPluginName.value = '';
}

async function confirmInstallUnsigned() {
  if (!confirmingPluginId.value) return;

  const pluginId = confirmingPluginId.value;
  showSignatureDialog.value = false;
  confirmingPluginId.value = null;
  confirmingPluginName.value = '';

  // 跳过签名验证重新安装
  try {
    await pluginStore.installMarketplacePlugin(pluginId, true);
  } catch (e) {
    console.error('Install failed:', e);
  }
}

async function installPlugin(pluginId: string) {
  const status = getPluginStatus(pluginId);

  if (status === 'installed' || status === 'downloading' || status === 'installing') {
    return;
  }

  // 如果是需要确认状态，显示对话框
  if (status === 'confirm') {
    const plugin = displayPlugins.value.find(p => p.id === pluginId);
    showSignatureConfirmDialog(pluginId, plugin?.name ?? pluginId);
    return;
  }

  if (status === 'error') {
    pluginStore.resetInstallStatus(pluginId);
  }

  try {
    const result = await pluginStore.installMarketplacePlugin(pluginId);

    // 如果需要签名确认，显示对话框
    if (result === 'need_confirm') {
      const plugin = displayPlugins.value.find(p => p.id === pluginId);
      showSignatureConfirmDialog(pluginId, plugin?.name ?? pluginId);
    }
  } catch (e) {
    console.error('Install failed:', e);
  }
}

// ============================================================================
// 辅助函数
// ============================================================================

function formatDownloads(count: number): string {
  if (count >= 10000) {
    return (count / 10000).toFixed(1) + '万';
  }
  return count.toLocaleString();
}

const isInSearchMode = computed(() => searchQuery.value.trim().length > 0);

const displayPlugins = computed(() => {
  if (isInSearchMode.value) {
    return searchResults.value;
  }
  return allPlugins.value;
});

const showAllPluginsTitle = computed(() => {
  return !isInSearchMode.value && !isSearching.value;
});

const showSearchTitle = computed(() => {
  return isInSearchMode.value && searchResults.value.length > 0 && !isSearching.value;
});

const showNoResults = computed(() => {
  return isInSearchMode.value && !isSearching.value && searchResults.value.length === 0;
});
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>插件市场</h2>
    </template>

    <div class="marketplace-page">
      <!-- 顶部操作栏 -->
      <div class="toolbar">
        <!-- 搜索框 -->
        <div class="search-box">
          <IconSearch class="search-icon" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索插件..."
            aria-label="搜索插件"
          >
        </div>

        <!-- 刷新按钮 -->
        <button
          class="refresh-btn"
          :class="{ 'is-refreshing': isRefreshing }"
          :disabled="isRefreshing || isLoadingPlugins"
          :title="formattedRefreshTime || '刷新插件列表'"
          @click="refreshPlugins"
        >
          <IconRefresh class="refresh-icon" />
          <span v-if="!isRefreshing">刷新</span>
          <span v-else>刷新中...</span>
        </button>
      </div>

      <!-- 加载状态 -->
      <div
        v-if="isLoadingPlugins"
        class="loading-state"
      >
        <span class="loading-spinner" />
        <span>正在从远程仓库加载插件...</span>
      </div>

      <!-- 加载错误 -->
      <div
        v-else-if="loadError"
        class="error-state"
      >
        <div class="error-icon">
          ⚠️
        </div>
        <p class="error-message">
          {{ loadError }}
        </p>
        <p class="error-hint">
          请检查网络连接或在设置中配置仓库地址
        </p>
        <button
          class="retry-btn"
          @click="refreshPlugins"
        >
          重试
        </button>
      </div>

      <!-- 空状态 -->
      <div
        v-else-if="allPlugins.length === 0 && !isInSearchMode"
        class="empty-state"
      >
        <div class="empty-icon">
          📦
        </div>
        <p class="empty-message">
          暂无可用插件
        </p>
        <p class="empty-hint">
          请检查仓库配置或稍后重试
        </p>
        <button
          class="retry-btn"
          @click="refreshPlugins"
        >
          刷新
        </button>
      </div>

      <!-- 插件列表 -->
      <template v-else>
        <!-- 全部插件标题 -->
        <div
          v-if="showAllPluginsTitle"
          class="section-header"
        >
          <h3 class="section-title">
            全部插件
          </h3>
          <span class="plugin-count">共 {{ allPlugins.length }} 个插件</span>
        </div>

        <!-- 搜索结果标题 -->
        <div
          v-else-if="showSearchTitle"
          class="section-header"
        >
          <h3 class="section-title">
            搜索结果
          </h3>
          <span class="result-count">{{ searchResults.length }} 个插件</span>
        </div>

        <!-- 搜索中 -->
        <div
          v-if="isSearching && searchQuery.trim()"
          class="search-status"
        >
          <div class="search-loading">
            <span class="loading-spinner" />
            <span>搜索中...</span>
          </div>
        </div>

        <!-- 插件网格 -->
        <div
          v-else-if="displayPlugins.length > 0"
          class="plugins-grid"
        >
          <div
            v-for="plugin in displayPlugins"
            :key="plugin.id"
            class="plugin-card"
          >
            <div class="plugin-header">
              <div class="plugin-icon">
                {{ plugin.icon }}
              </div>
              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{{ plugin.name }}</span>
                  <IconVerified
                    v-if="plugin.verified"
                    class="verified-badge"
                  />
                </div>
                <p class="plugin-description">
                  {{ plugin.description }}
                </p>
              </div>
              <button
                class="install-btn"
                :class="{
                  'is-installed': getPluginStatus(plugin.id) === 'installed',
                  'is-loading': ['downloading', 'installing'].includes(getPluginStatus(plugin.id)),
                  'is-success': getPluginStatus(plugin.id) === 'success',
                  'is-error': getPluginStatus(plugin.id) === 'error',
                }"
                :disabled="isButtonDisabled(plugin.id)"
                :aria-label="'安装 ' + plugin.name"
                @click="installPlugin(plugin.id)"
              >
                <IconDownload v-if="shouldShowIcon(plugin.id)" />
                <span
                  v-else
                  class="btn-text"
                >{{ getButtonText(plugin.id) }}</span>
              </button>
            </div>
            <div class="plugin-footer">
              <span class="plugin-author">{{ plugin.author }}</span>
              <span class="plugin-separator">•</span>
              <span class="plugin-downloads">{{ formatDownloads(plugin.downloads) }} 下载</span>
              <span class="plugin-separator">•</span>
              <span class="plugin-version">v{{ plugin.version }}</span>
            </div>
            <!-- 安装错误提示 -->
            <div
              v-if="pluginStore.getInstallError(plugin.id)"
              class="install-error"
            >
              {{ pluginStore.getInstallError(plugin.id) }}
            </div>
          </div>
        </div>

        <!-- 搜索无结果 -->
        <div
          v-if="showNoResults"
          class="search-status"
        >
          <div class="search-empty">
            <span class="empty-icon">🔍</span>
            <p>未找到匹配 "{{ searchQuery }}" 的插件</p>
          </div>
        </div>
      </template>
    </div>

    <!-- 签名确认对话框 -->
    <Teleport to="body">
      <div
        v-if="showSignatureDialog"
        class="dialog-overlay"
        @click.self="closeSignatureDialog"
      >
        <div class="dialog-content">
          <div class="dialog-icon">
            ⚠️
          </div>
          <h3 class="dialog-title">
            安装未签名插件
          </h3>
          <p class="dialog-message">
            插件 <strong>{{ confirmingPluginName }}</strong> 未经过官方签名验证。
          </p>
          <p class="dialog-warning">
            未签名的插件可能存在安全风险，请确保您信任该插件的来源。
          </p>
          <div class="dialog-actions">
            <button
              class="dialog-btn dialog-btn-cancel"
              @click="closeSignatureDialog"
            >
              取消
            </button>
            <button
              class="dialog-btn dialog-btn-confirm"
              @click="confirmInstallUnsigned"
            >
              仍然安装
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </AppLayout>
</template>

<style scoped>
.marketplace-page {
  width: 100%;
  max-width: 900px;
  box-sizing: border-box;
}

/* 顶部操作栏 */
.toolbar {
  display: flex;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
  flex: 1;
}

.search-icon {
  position: absolute;
  left: var(--spacing-md);
  color: var(--color-text-tertiary);
}

.search-box input {
  width: 100%;
  padding: var(--spacing-md) var(--spacing-md) var(--spacing-md) 48px;
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  font-size: 0.9375rem;
  color: var(--color-text);
  transition: all var(--transition-fast);
}

.search-box input::placeholder {
  color: var(--color-text-tertiary);
}

.search-box input:focus {
  outline: none;
  border-color: var(--color-accent);
}

/* 刷新按钮 */
.refresh-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  color: var(--color-text-secondary);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--color-bg-hover);
  color: var(--color-text);
  border-color: var(--color-accent);
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.refresh-btn.is-refreshing .refresh-icon {
  animation: spin 1s linear infinite;
}

.refresh-icon {
  width: 16px;
  height: 16px;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.plugin-count {
  background: var(--color-bg-tertiary);
  padding: 2px var(--spacing-sm);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

/* 区块标题 */
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-lg);
}

.section-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text-secondary);
  margin: 0;
}

.result-count {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

/* 插件网格 */
.plugins-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-md);
  width: 100%;
  box-sizing: border-box;
}

.plugin-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  transition: background var(--transition-fast);
  box-sizing: border-box;
  overflow: hidden;
  min-width: 0;
}

.plugin-card:hover {
  background: var(--color-bg-hover);
}

.plugin-header {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.plugin-icon {
  width: 40px;
  height: 40px;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 1rem;
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  margin-bottom: var(--spacing-xs);
}

.plugin-name {
  font-weight: 600;
  color: var(--color-text);
}

.verified-badge {
  flex-shrink: 0;
}

.plugin-description {
  font-size: 0.8125rem;
  color: var(--color-text-secondary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.install-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  flex-shrink: 0;
  min-width: 36px;
  min-height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.install-btn:hover:not(:disabled) {
  background: var(--color-bg-tertiary);
  color: var(--color-text);
}

.install-btn:disabled {
  cursor: default;
}

.install-btn.is-installed {
  color: var(--color-success);
  font-size: 0.75rem;
}

.install-btn.is-loading {
  color: var(--color-accent);
  font-size: 0.75rem;
}

.install-btn.is-success {
  color: var(--color-success);
  font-size: 0.75rem;
}

.install-btn.is-error {
  color: var(--color-error);
}

.btn-text {
  white-space: nowrap;
}

.plugin-footer {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.plugin-author {
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-separator {
  color: var(--color-border-light);
}

.install-error {
  margin-top: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  background: color-mix(in srgb, var(--color-error) 10%, transparent);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--color-error);
}

/* 加载状态 */
.loading-state {
  padding: var(--spacing-xxl);
  text-align: center;
  color: var(--color-text-secondary);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-md);
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* 错误状态 */
.error-state {
  padding: var(--spacing-xxl) var(--spacing-xxl);
  margin: var(--spacing-xxl) 0;
  text-align: center;
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
}

.error-icon {
  font-size: 2.5rem;
  margin-bottom: var(--spacing-md);
}

.error-message {
  font-size: 1rem;
  color: var(--color-text);
  margin: 0 0 var(--spacing-xs);
}

.error-hint {
  font-size: 0.875rem;
  color: var(--color-text-tertiary);
  margin: 0 0 var(--spacing-lg);
}

.retry-btn {
  padding: var(--spacing-sm) var(--spacing-xl);
  margin-bottom: var(--spacing-lg);
  background: var(--color-accent);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.875rem;
  font-weight: 500;
  transition: opacity var(--transition-fast);
}

.retry-btn:hover {
  opacity: 0.9;
}

/* 空状态 */
.empty-state {
  padding: var(--spacing-xxl);
  text-align: center;
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
}

.empty-icon {
  font-size: 2.5rem;
  margin-bottom: var(--spacing-md);
  opacity: 0.6;
}

.empty-message {
  font-size: 1rem;
  color: var(--color-text);
  margin: 0 0 var(--spacing-xs);
}

.empty-hint {
  font-size: 0.875rem;
  color: var(--color-text-tertiary);
  margin: 0 0 var(--spacing-lg);
}

/* 搜索状态 */
.search-status {
  padding: var(--spacing-xl);
  text-align: center;
}

.search-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-sm);
  color: var(--color-text-secondary);
  font-size: 0.875rem;
}

.search-empty {
  color: var(--color-text-secondary);
}

.search-empty .empty-icon {
  font-size: 2rem;
  display: block;
  margin-bottom: var(--spacing-sm);
  opacity: 0.6;
}

.search-empty p {
  margin: 0;
  font-size: 0.875rem;
}

/* 响应式 */
@media (max-width: 600px) {
  .plugins-grid {
    grid-template-columns: 1fr;
  }

  .toolbar {
    flex-direction: column;
  }

  .refresh-btn {
    width: 100%;
    justify-content: center;
  }
}

/* 确认对话框 */
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.dialog-content {
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  padding: var(--spacing-xl);
  max-width: 400px;
  width: 90%;
  text-align: center;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
}

.dialog-icon {
  font-size: 2.5rem;
  margin-bottom: var(--spacing-md);
}

.dialog-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0 0 var(--spacing-md);
}

.dialog-message {
  font-size: 0.9375rem;
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-sm);
  line-height: 1.5;
}

.dialog-message strong {
  color: var(--color-text);
}

.dialog-warning {
  font-size: 0.8125rem;
  color: var(--color-warning, #f59e0b);
  margin: 0 0 var(--spacing-lg);
  padding: var(--spacing-sm) var(--spacing-md);
  background: color-mix(in srgb, var(--color-warning, #f59e0b) 10%, transparent);
  border-radius: var(--radius-md);
}

.dialog-actions {
  display: flex;
  gap: var(--spacing-md);
  justify-content: center;
}

.dialog-btn {
  padding: var(--spacing-sm) var(--spacing-xl);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  border: none;
}

.dialog-btn-cancel {
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
}

.dialog-btn-cancel:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.dialog-btn-confirm {
  background: var(--color-warning, #f59e0b);
  color: white;
}

.dialog-btn-confirm:hover {
  opacity: 0.9;
}
</style>
