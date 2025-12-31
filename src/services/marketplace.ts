// MarketplaceService - 插件市场数据服务
// 管理插件仓库索引、搜索和安装

import type { MarketplacePlugin, PluginRegistry, PluginInfo, Result } from '@/types';

// 远程仓库 URL（默认地址，可通过 setRegistryUrl 覆盖）
const DEFAULT_REGISTRY_URL = 'https://raw.githubusercontent.com/DDG0808/aibal-plugins/main/registry.json';
const STORAGE_KEY = 'marketplace_registry_url';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// ============================================================================
// MarketplaceService 类
// ============================================================================

/**
 * 插件市场服务
 * 提供插件仓库索引管理、搜索和安装功能
 * 全部从远程仓库获取，无内置插件
 */
class MarketplaceService {
  private registry: PluginRegistry | null = null;
  private isLoading = false;
  private lastFetchTime: number = 0;
  private lastError: string | null = null;
  private readonly CACHE_TTL = 5 * 60 * 1000; // 5 分钟缓存

  /**
   * 获取仓库索引
   * 全部从远程获取，无内置数据
   */
  async getRegistry(forceRefresh = false): Promise<PluginRegistry> {
    const now = Date.now();
    const cacheExpired = now - this.lastFetchTime > this.CACHE_TTL;

    // 有缓存且未过期，直接返回
    if (!forceRefresh && !cacheExpired && this.registry) {
      return this.registry;
    }

    // 需要刷新
    await this.fetchRemoteRegistry();

    // 如果获取失败且没有缓存，返回空仓库
    if (!this.registry) {
      return {
        version: '0.0.0',
        lastUpdated: new Date().toISOString(),
        featured: [],
        plugins: [],
      };
    }

    return this.registry;
  }

  /**
   * 强制刷新仓库
   */
  async refreshRegistry(): Promise<{ success: boolean; error?: string }> {
    this.lastError = null;
    await this.fetchRemoteRegistry();

    if (this.lastError) {
      return { success: false, error: this.lastError };
    }
    return { success: true };
  }

  /**
   * 获取加载状态
   */
  getLoadingState(): boolean {
    return this.isLoading;
  }

  /**
   * 获取最后错误
   */
  getLastError(): string | null {
    return this.lastError;
  }

  /**
   * 获取当前仓库 URL
   */
  getRegistryUrl(): string {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) return saved;
    }
    return DEFAULT_REGISTRY_URL;
  }

  /**
   * 设置自定义仓库 URL
   * @param url 自定义 URL，传入空字符串或 null 恢复默认
   */
  setRegistryUrl(url: string | null): void {
    if (typeof localStorage !== 'undefined') {
      if (url && url.trim()) {
        localStorage.setItem(STORAGE_KEY, url.trim());
      } else {
        localStorage.removeItem(STORAGE_KEY);
      }
    }
    // 清除缓存，下次获取时会使用新 URL
    this.lastFetchTime = 0;
    this.registry = null;
  }

  /**
   * 从远程获取仓库索引
   */
  private async fetchRemoteRegistry(): Promise<void> {
    if (this.isLoading) return;
    this.isLoading = true;
    this.lastError = null;

    try {
      const registryUrl = this.getRegistryUrl();
      const response = await fetch(registryUrl, {
        method: 'GET',
        headers: { 'Accept': 'application/json' },
        cache: 'no-cache',
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json() as PluginRegistry;

      // 验证并规范化数据
      this.registry = this.validateAndNormalizeRegistry(data);
      this.lastFetchTime = Date.now();
    } catch (e) {
      this.lastError = e instanceof Error ? e.message : '获取仓库失败';
      console.error('[Marketplace] 远程仓库获取失败:', e);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 验证并规范化仓库数据
   */
  private validateAndNormalizeRegistry(data: unknown): PluginRegistry {
    const raw = data as Partial<PluginRegistry>;
    const plugins: MarketplacePlugin[] = [];

    // 验证 plugins 数组
    if (Array.isArray(raw.plugins)) {
      for (const plugin of raw.plugins) {
        if (this.isValidPlugin(plugin)) {
          plugins.push(this.normalizePlugin(plugin));
        }
      }
    }

    // 验证 featured 数组
    const featured: string[] = [];
    if (Array.isArray(raw.featured)) {
      for (const id of raw.featured) {
        if (typeof id === 'string' && id && plugins.some(p => p.id === id)) {
          featured.push(id);
        }
      }
    }

    return {
      version: typeof raw.version === 'string' ? raw.version : '1.0.0',
      lastUpdated: typeof raw.lastUpdated === 'string' ? raw.lastUpdated : new Date().toISOString(),
      featured,
      plugins,
    };
  }

  /**
   * 验证插件数据是否有效
   */
  private isValidPlugin(plugin: unknown): plugin is MarketplacePlugin {
    if (!plugin || typeof plugin !== 'object') return false;
    const p = plugin as Record<string, unknown>;
    return (
      typeof p.id === 'string' && p.id.length > 0 &&
      typeof p.name === 'string' && p.name.length > 0 &&
      typeof p.description === 'string' &&
      typeof p.author === 'string' &&
      typeof p.version === 'string'
    );
  }

  /**
   * 规范化插件数据
   */
  private normalizePlugin(plugin: MarketplacePlugin): MarketplacePlugin {
    return {
      ...plugin,
      downloads: typeof plugin.downloads === 'number' ? plugin.downloads : 0,
      verified: typeof plugin.verified === 'boolean' ? plugin.verified : false,
      icon: typeof plugin.icon === 'string' ? plugin.icon : plugin.name.charAt(0).toUpperCase(),
      tags: Array.isArray(plugin.tags)
        ? plugin.tags.filter((t): t is string => typeof t === 'string')
        : [],
    };
  }

  /**
   * 获取热门插件
   */
  async getFeaturedPlugins(): Promise<MarketplacePlugin[]> {
    const registry = await this.getRegistry();
    if (registry.featured.length === 0) {
      // 没有 featured 列表，返回前 6 个
      return registry.plugins.slice(0, 6);
    }
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
   *
   * @param pluginId 插件 ID
   * @param skipSignature 是否跳过签名验证（用户确认后传 true）
   */
  async installPlugin(pluginId: string, skipSignature = false): Promise<Result<PluginInfo>> {
    const plugin = await this.getPluginDetails(pluginId);
    if (!plugin) {
      return {
        success: false,
        error: { code: 'NOT_FOUND', message: `插件 ${pluginId} 不存在` },
      };
    }

    try {
      if (!isTauri) {
        // 浏览器环境模拟安装
        await new Promise(resolve => setTimeout(resolve, 1500));

        // 模拟签名验证失败（用于测试确认对话框）
        if (!skipSignature && !plugin.verified) {
          return {
            success: false,
            error: {
              code: 'SIGNATURE_INVALID',
              message: '插件未签名或签名无效'
            },
          };
        }

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

      // Tauri 环境调用后端
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<Result<PluginInfo>>('plugin_install', {
        source: plugin.downloadUrl || `registry://${pluginId}`,
        skipSignature: skipSignature,
        registryUrl: this.getRegistryUrl(),
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
   * 检查插件是否需要跳过签名验证
   * 用于判断是否需要显示确认对话框
   */
  isSignatureError(errorCode: string | undefined): boolean {
    return errorCode === 'SIGNATURE_INVALID' || errorCode === 'SIGNATURE_MISSING';
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
