// MarketplaceService - 插件市场数据服务
// 管理插件仓库索引、搜索和安装

import type { MarketplacePlugin, PluginRegistry, PluginInfo, Result } from '@/types';

// ============================================================================
// 内置插件仓库索引（默认数据）
// ============================================================================

const BUILTIN_REGISTRY: PluginRegistry = {
  version: '1.0.0',
  lastUpdated: new Date().toISOString(),
  featured: [],
  plugins: [],
};

// 默认远程仓库 URL
// 开发环境使用本地 public/registry.json，生产环境使用 GitHub
const DEFAULT_REGISTRY_URL = import.meta.env.DEV
  ? '/registry.json'
  : 'https://raw.githubusercontent.com/cuk-team/cuk-plugins/main/registry.json';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// ============================================================================
// MarketplaceService 类
// ============================================================================

/**
 * 插件市场服务
 * 提供插件仓库索引管理、搜索和安装功能
 */
class MarketplaceService {
  private registry: PluginRegistry = BUILTIN_REGISTRY;
  private isLoading = false;
  private lastFetchTime: number = 0;
  private readonly CACHE_TTL = 5 * 60 * 1000; // 5 分钟缓存
  private customRegistryUrl: string | null = null;

  /**
   * 获取当前市场 URL
   */
  getRegistryUrl(): string {
    return this.customRegistryUrl || DEFAULT_REGISTRY_URL;
  }

  /**
   * 获取默认市场 URL
   */
  getDefaultRegistryUrl(): string {
    return DEFAULT_REGISTRY_URL;
  }

  /**
   * 设置自定义市场 URL
   * @param url 新的市场 URL，传 null 或空字符串恢复默认
   */
  setRegistryUrl(url: string | null): void {
    const newUrl = url?.trim() || null;
    if (newUrl !== this.customRegistryUrl) {
      this.customRegistryUrl = newUrl;
      // 清除缓存，强制下次刷新
      this.lastFetchTime = 0;
      console.info('[Marketplace] 市场 URL 已更新:', this.getRegistryUrl());
    }
  }

  /**
   * 获取仓库索引
   * 首次加载等待远程数据，后续使用缓存
   */
  async getRegistry(forceRefresh = false): Promise<PluginRegistry> {
    const now = Date.now();
    const cacheExpired = now - this.lastFetchTime > this.CACHE_TTL;
    const isFirstLoad = this.lastFetchTime === 0;

    if (!forceRefresh && !cacheExpired) {
      return this.registry;
    }

    // 首次加载或强制刷新时，等待远程数据
    if ((isFirstLoad || forceRefresh) && !this.isLoading) {
      try {
        await this.fetchRemoteRegistry();
      } catch (e) {
        console.warn('[Marketplace] 远程仓库获取失败，使用内置数据:', e);
      }
    } else if (!this.isLoading) {
      // 后续刷新非阻塞
      this.fetchRemoteRegistry().catch(e => {
        console.warn('[Marketplace] 远程仓库刷新失败:', e);
      });
    }

    return this.registry;
  }

  /**
   * 从远程获取仓库索引
   */
  private async fetchRemoteRegistry(): Promise<void> {
    if (this.isLoading) return;
    this.isLoading = true;

    try {
      const url = this.getRegistryUrl();
      // 在 Tauri 环境中使用 fetch（需要配置 allowlist）
      // 在浏览器环境中直接使用 fetch
      const response = await fetch(url, {
        method: 'GET',
        headers: { 'Accept': 'application/json' },
      });

      if (response.ok) {
        const data = await response.json() as PluginRegistry;
        // 合并远程数据和内置数据（保留内置插件）
        this.registry = this.mergeRegistry(data);
        this.lastFetchTime = Date.now();
      }
    } catch {
      // 静默失败，继续使用内置数据
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 合并远程和内置仓库
   * 添加运行时校验，确保远程数据结构正确
   */
  private mergeRegistry(remote: Partial<PluginRegistry>): PluginRegistry {
    const pluginMap = new Map<string, MarketplacePlugin>();

    // 运行时校验：确保 plugins 是数组
    const remotePlugins = Array.isArray(remote.plugins) ? remote.plugins : [];
    const remoteFeatured = Array.isArray(remote.featured) ? remote.featured : [];

    // 先添加远程插件（过滤无效条目）
    // 校验所有必填字段，避免后续使用时 TypeError：
    // - searchPlugins(): name/description/author.toLowerCase()
    // - compareVersions(): version.split()
    // - formatDownloads(): downloads.toLocaleString()
    // - UI 渲染: verified 布尔值
    for (const plugin of remotePlugins) {
      if (
        plugin &&
        typeof plugin.id === 'string' && plugin.id &&
        typeof plugin.name === 'string' && plugin.name &&
        typeof plugin.description === 'string' &&
        typeof plugin.author === 'string' && plugin.author &&
        typeof plugin.version === 'string' && plugin.version &&
        typeof plugin.downloads === 'number' &&
        typeof plugin.verified === 'boolean'
      ) {
        // 归一化 tags：确保是 string[] 或 undefined
        const normalizedPlugin = { ...plugin };
        if (normalizedPlugin.tags !== undefined) {
          if (!Array.isArray(normalizedPlugin.tags)) {
            normalizedPlugin.tags = [];
          } else {
            normalizedPlugin.tags = normalizedPlugin.tags.filter(
              (tag): tag is string => typeof tag === 'string'
            );
          }
        }
        pluginMap.set(plugin.id, normalizedPlugin);
      }
    }

    // 内置插件无条件保留（覆盖远程同 ID 插件）
    // 内置插件本身可信，verified 字段仅用于 UI 展示"官方认证"标识
    for (const plugin of BUILTIN_REGISTRY.plugins) {
      pluginMap.set(plugin.id, plugin);
    }

    // 过滤有效的 featured ID
    const validFeatured = remoteFeatured.filter(id => typeof id === 'string' && id);

    return {
      version: (typeof remote.version === 'string' && remote.version) || BUILTIN_REGISTRY.version,
      lastUpdated: (typeof remote.lastUpdated === 'string' && remote.lastUpdated) || new Date().toISOString(),
      featured: [...new Set([...validFeatured, ...BUILTIN_REGISTRY.featured])],
      plugins: Array.from(pluginMap.values()),
    };
  }

  /**
   * 获取热门插件
   */
  async getFeaturedPlugins(): Promise<MarketplacePlugin[]> {
    const registry = await this.getRegistry();
    return registry.featured
      .map(id => registry.plugins.find(p => p.id === id))
      .filter((p): p is MarketplacePlugin => p !== undefined);
  }

  /**
   * 获取所有插件
   */
  async getAllPlugins(): Promise<MarketplacePlugin[]> {
    const registry = await this.getRegistry();
    return registry.plugins;
  }

  /**
   * 搜索插件
   */
  async searchPlugins(query: string): Promise<MarketplacePlugin[]> {
    if (!query.trim()) {
      return [];
    }

    const registry = await this.getRegistry();
    const lowerQuery = query.toLowerCase();

    return registry.plugins.filter(plugin => {
      return (
        plugin.name.toLowerCase().includes(lowerQuery) ||
        plugin.description.toLowerCase().includes(lowerQuery) ||
        plugin.author.toLowerCase().includes(lowerQuery) ||
        plugin.tags?.some(tag => tag.toLowerCase().includes(lowerQuery))
      );
    });
  }

  /**
   * 获取插件详情
   */
  async getPluginDetails(id: string): Promise<MarketplacePlugin | null> {
    const registry = await this.getRegistry();
    return registry.plugins.find(p => p.id === id) ?? null;
  }

  /**
   * 按类别筛选
   */
  async getPluginsByCategory(category: 'usage' | 'balance' | 'status' | 'custom'): Promise<MarketplacePlugin[]> {
    const registry = await this.getRegistry();
    return registry.plugins.filter(p => p.dataType === category);
  }

  /**
   * 获取认证插件
   */
  async getVerifiedPlugins(): Promise<MarketplacePlugin[]> {
    const registry = await this.getRegistry();
    return registry.plugins.filter(p => p.verified);
  }

  /**
   * 安装插件
   * 调用后端 plugin_install 命令
   * 返回类型与 IPC 契约对齐：Result<PluginInfo>
   */
  async installPlugin(pluginId: string): Promise<Result<PluginInfo>> {
    const plugin = await this.getPluginDetails(pluginId);
    if (!plugin) {
      return {
        success: false,
        error: { code: 'NOT_FOUND', message: `插件 ${pluginId} 不存在` },
      };
    }

    try {
      if (!isTauri) {
        // 浏览器环境模拟安装，返回模拟的 PluginInfo
        await new Promise(resolve => setTimeout(resolve, 1500));
        const mockPluginInfo: PluginInfo = {
          id: plugin.id,
          name: plugin.name,
          version: plugin.version,
          pluginType: plugin.pluginType ?? 'data',
          dataType: plugin.dataType,
          enabled: true,
          healthy: true,
          author: plugin.author,
          description: plugin.description,
          icon: plugin.icon,
        };
        return { success: true, data: mockPluginInfo };
      }

      // Tauri 环境调用后端（契约返回 Result<PluginInfo>）
      const { invoke } = await import('@tauri-apps/api/core');

      // 处理 downloadUrl：如果是相对路径，补全为完整 URL
      let source = plugin.downloadUrl || `registry://${pluginId}`;
      if (source.startsWith('/')) {
        // 相对路径，使用当前页面的 origin 补全
        source = `${window.location.origin}${source}`;
      }

      // 处理 registryUrl：如果是相对路径，补全为完整 URL
      let registryUrl = this.getRegistryUrl();
      if (registryUrl.startsWith('/')) {
        registryUrl = `${window.location.origin}${registryUrl}`;
      }

      const result = await invoke<Result<PluginInfo>>('plugin_install', {
        source,
        skipSignature: !plugin.verified,
        registryUrl, // 传递完整的市场 URL
      });

      return result;
    } catch (e) {
      const message = e instanceof Error ? e.message : '安装失败';
      return {
        success: false,
        error: { code: 'INSTALL_ERROR', message },
      };
    }
  }

  /**
   * 检查插件是否有更新
   */
  async checkPluginUpdate(installedId: string, installedVersion: string): Promise<{
    hasUpdate: boolean;
    latestVersion?: string;
  }> {
    const plugin = await this.getPluginDetails(installedId);
    if (!plugin) {
      return { hasUpdate: false };
    }

    const hasUpdate = this.compareVersions(plugin.version, installedVersion) > 0;
    return {
      hasUpdate,
      latestVersion: hasUpdate ? plugin.version : undefined,
    };
  }

  /**
   * 比较版本号
   * 返回: 1 (a > b), 0 (a == b), -1 (a < b)
   */
  private compareVersions(a: string, b: string): number {
    const partsA = a.split('.').map(Number);
    const partsB = b.split('.').map(Number);
    const maxLen = Math.max(partsA.length, partsB.length);

    for (let i = 0; i < maxLen; i++) {
      const numA = partsA[i] || 0;
      const numB = partsB[i] || 0;
      if (numA > numB) return 1;
      if (numA < numB) return -1;
    }

    return 0;
  }
}

// 导出单例
export const marketplaceService = new MarketplaceService();
