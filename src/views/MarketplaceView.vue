<script setup lang="ts">
/**
 * 插件市场视图
 * Phase 8.2: 搜索和安装社区插件（对接真实功能）
 */
import { ref, watch, onMounted, onUnmounted, computed } from 'vue';
import { AppLayout } from '@/components/layout';
import { IconSearch, IconDownload, IconVerified } from '@/components/icons';
import { usePluginStore } from '@/stores';
import { marketplaceService } from '@/services';
import type { MarketplacePlugin, InstallStatus } from '@/types';

const pluginStore = usePluginStore();

// debounce timer（用于清理）
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

// debounce 工具函数（支持清理）
function debounceFn<T extends (...args: unknown[]) => void>(fn: T, delay: number): T {
  return ((...args: unknown[]) => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => fn(...args), delay);
  }) as T;
}

// 组件卸载时清理 debounce timer
onUnmounted(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
});

// 加载状态
const isLoadingFeatured = ref(true);
const loadError = ref<string | null>(null);

// 搜索关键词
const searchQuery = ref('');
const isSearching = ref(false);

// 热门插件（从服务获取）
const featuredPlugins = ref<MarketplacePlugin[]>([]);

// 搜索结果
const searchResults = ref<MarketplacePlugin[]>([]);

// 初始化加载热门插件
onMounted(async () => {
  try {
    isLoadingFeatured.value = true;
    loadError.value = null;
    featuredPlugins.value = await marketplaceService.getFeaturedPlugins();
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : '加载失败';
    console.error('Failed to load featured plugins:', e);
  } finally {
    isLoadingFeatured.value = false;
  }
});

// 执行搜索（实际搜索逻辑）
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

// 使用 debounce 包装搜索（300ms 延迟）
const debouncedSearch = debounceFn(performSearch, 300);

// 监听搜索输入变化（清空时立即处理，避免闪烁）
watch(searchQuery, (newVal) => {
  if (!newVal.trim()) {
    // 立即清空结果，不走 debounce，避免短暂显示旧结果
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

// 获取插件安装状态
function getPluginStatus(pluginId: string): InstallStatus | 'installed' {
  if (pluginStore.isInstalled(pluginId)) {
    return 'installed';
  }
  return pluginStore.getInstallStatus(pluginId);
}

// 获取按钮文本
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
    default:
      return '';
  }
}

// 检查按钮是否禁用
function isButtonDisabled(pluginId: string): boolean {
  const status = getPluginStatus(pluginId);
  return status === 'installed' || status === 'downloading' || status === 'installing' || status === 'success';
}

// 检查是否显示图标
function shouldShowIcon(pluginId: string): boolean {
  const status = getPluginStatus(pluginId);
  return status === 'idle' || status === 'error';
}

// 安装插件
async function installPlugin(pluginId: string) {
  const status = getPluginStatus(pluginId);

  // 已安装或正在安装，不处理
  if (status === 'installed' || status === 'downloading' || status === 'installing') {
    return;
  }

  // 错误状态，重置后重试
  if (status === 'error') {
    pluginStore.resetInstallStatus(pluginId);
  }

  try {
    await pluginStore.installMarketplacePlugin(pluginId);
  } catch (e) {
    console.error('Install failed:', e);
  }
}

// 格式化下载数
function formatDownloads(count: number): string {
  if (count >= 10000) {
    return (count / 10000).toFixed(1) + '万';
  }
  return count.toLocaleString();
}

// 是否处于搜索模式
const isInSearchMode = computed(() => searchQuery.value.trim().length > 0);

// 计算要显示的插件列表（修复：搜索模式下始终返回搜索结果，即使为空）
const displayPlugins = computed(() => {
  if (isInSearchMode.value) {
    return searchResults.value; // 搜索模式下返回结果（可能为空）
  }
  return featuredPlugins.value;
});

// 是否显示热门区域标题
const showFeaturedTitle = computed(() => {
  return !isInSearchMode.value && !isSearching.value;
});

// 是否显示搜索结果标题
const showSearchTitle = computed(() => {
  return isInSearchMode.value && searchResults.value.length > 0 && !isSearching.value;
});

// 是否显示搜索无结果
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
      <!-- 搜索框 -->
      <div class="search-section">
        <div class="search-box">
          <IconSearch class="search-icon" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索社区插件..."
            aria-label="搜索社区插件"
          >
        </div>
      </div>

      <!-- 加载状态 -->
      <div
        v-if="isLoadingFeatured"
        class="loading-state"
      >
        <span class="loading-spinner" />
        <span>加载插件列表...</span>
      </div>

      <!-- 加载错误 -->
      <div
        v-else-if="loadError"
        class="error-state"
      >
        <p>{{ loadError }}</p>
        <button
          class="retry-btn"
          @click="$router.go(0)"
        >
          重试
        </button>
      </div>

      <!-- 插件列表 -->
      <template v-else>
        <!-- 热门推荐标题 -->
        <div
          v-if="showFeaturedTitle"
          class="section-header"
        >
          <h2 class="section-title">
            热门推荐
          </h2>
        </div>

        <!-- 搜索结果标题 -->
        <div
          v-else-if="showSearchTitle"
          class="section-header"
        >
          <h2 class="section-title">
            搜索结果
          </h2>
          <span class="result-count">{{ searchResults.length }} 个插件</span>
        </div>

        <!-- 搜索状态：加载中 -->
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
  </AppLayout>
</template>

<style scoped>
.marketplace-page {
  max-width: 900px;
}

.search-section {
  margin-bottom: var(--spacing-xl);
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
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

.plugins-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-md);
}

.plugin-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  transition: background var(--transition-fast);
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

/* 加载和错误状态 */
.loading-state,
.error-state {
  padding: var(--spacing-xl);
  text-align: center;
  color: var(--color-text-secondary);
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-sm);
}

.retry-btn {
  margin-top: var(--spacing-md);
  padding: var(--spacing-sm) var(--spacing-lg);
  background: var(--color-accent);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.875rem;
}

.retry-btn:hover {
  opacity: 0.9;
}

/* 搜索状态样式 */
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

.loading-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
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

/* 响应式：小屏幕单列 */
@media (max-width: 600px) {
  .plugins-grid {
    grid-template-columns: 1fr;
  }
}
</style>
