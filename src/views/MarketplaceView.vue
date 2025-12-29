<script setup lang="ts">
/**
 * 插件市场视图
 * Phase 8.2: 搜索和安装社区插件
 */
import { ref } from 'vue';
import { AppLayout } from '@/components/layout';

interface MarketplacePlugin {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  downloads: number;
  verified: boolean;
  icon?: string;
}

// 搜索关键词
const searchQuery = ref('');

// 模拟热门插件
const featuredPlugins = ref<MarketplacePlugin[]>([
  {
    id: 'midjourney-stats',
    name: 'Midjourney 统计',
    description: '追踪剩余快速模式时长',
    author: 'Community',
    version: '1.0.4',
    downloads: 12000,
    verified: true,
    icon: 'M',
  },
  {
    id: 'copilot-usage',
    name: 'Copilot 用量',
    description: '企业席位利用率监控',
    author: 'CUK Official',
    version: '1.0.4',
    downloads: 8500,
    verified: true,
    icon: 'C',
  },
  {
    id: 'hf-status',
    name: 'HF 服务状态',
    description: '模型托管服务状态',
    author: 'Community',
    version: '1.0.4',
    downloads: 3200,
    verified: false,
    icon: 'H',
  },
]);

// 搜索结果
const searchResults = ref<MarketplacePlugin[]>([]);

// 执行搜索
function handleSearch() {
  // TODO: 实现搜索逻辑
  if (!searchQuery.value.trim()) {
    searchResults.value = [];
    return;
  }
  // 模拟搜索
  searchResults.value = featuredPlugins.value.filter(
    p => p.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
         p.description.toLowerCase().includes(searchQuery.value.toLowerCase())
  );
}

// 安装插件
async function installPlugin(id: string) {
  // TODO: 调用 IPC plugin_install
  console.log('Installing plugin:', id);
}

// 格式化下载数
function formatDownloads(count: number): string {
  if (count >= 10000) {
    return (count / 10000).toFixed(1) + '万';
  }
  return count.toString();
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h1>插件市场</h1>
    </template>

    <div class="marketplace-page">
      <!-- 搜索框 -->
      <div class="search-section">
        <div class="search-box">
          <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none">
            <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="2"/>
            <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索社区插件..."
            @input="handleSearch"
          >
        </div>
      </div>

      <!-- 热门推荐 -->
      <div class="featured-section">
        <h2 class="section-title">热门推荐</h2>

        <div class="plugins-grid">
          <div v-for="plugin in featuredPlugins" :key="plugin.id" class="plugin-card">
            <div class="plugin-header">
              <div class="plugin-icon">{{ plugin.icon }}</div>
              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{{ plugin.name }}</span>
                  <svg v-if="plugin.verified" class="verified-badge" width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path d="M22 11.08V12a10 10 0 11-5.93-9.14" stroke="var(--color-accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="22,4 12,14.01 9,11.01" stroke="var(--color-accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </div>
                <p class="plugin-description">{{ plugin.description }}</p>
              </div>
              <button class="download-btn" @click="installPlugin(plugin.id)">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
                  <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <polyline points="7,10 12,15 17,10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <line x1="12" y1="15" x2="12" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
              </button>
            </div>
            <div class="plugin-footer">
              <span class="plugin-downloads">{{ formatDownloads(plugin.downloads) }} 下载</span>
              <span class="plugin-separator">•</span>
              <span class="plugin-version">v{{ plugin.version }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 搜索结果 -->
      <div v-if="searchResults.length > 0" class="search-results">
        <h2 class="section-title">搜索结果</h2>
        <div class="plugins-grid">
          <div v-for="plugin in searchResults" :key="plugin.id" class="plugin-card">
            <div class="plugin-header">
              <div class="plugin-icon">{{ plugin.icon }}</div>
              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{{ plugin.name }}</span>
                  <svg v-if="plugin.verified" class="verified-badge" width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path d="M22 11.08V12a10 10 0 11-5.93-9.14" stroke="var(--color-accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="22,4 12,14.01 9,11.01" stroke="var(--color-accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </div>
                <p class="plugin-description">{{ plugin.description }}</p>
              </div>
              <button class="download-btn" @click="installPlugin(plugin.id)">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
                  <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <polyline points="7,10 12,15 17,10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <line x1="12" y1="15" x2="12" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
              </button>
            </div>
            <div class="plugin-footer">
              <span class="plugin-downloads">{{ formatDownloads(plugin.downloads) }} 下载</span>
              <span class="plugin-separator">•</span>
              <span class="plugin-version">v{{ plugin.version }}</span>
            </div>
          </div>
        </div>
      </div>
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

.featured-section,
.search-results {
  margin-bottom: var(--spacing-xl);
}

.section-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-lg);
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

.download-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.download-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text);
}

.plugin-footer {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.plugin-separator {
  color: var(--color-border-light);
}
</style>
