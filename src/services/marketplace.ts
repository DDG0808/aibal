// MarketplaceService - 插件市场数据服务
// 管理插件仓库索引、搜索和安装

import type { MarketplacePlugin, PluginRegistry, PluginInfo, Result } from '@/types';

// ============================================================================
// 内置插件仓库索引（默认数据）
// ============================================================================

const BUILTIN_REGISTRY: PluginRegistry = {
  version: '1.0.0',
  lastUpdated: new Date().toISOString(),
  featured: ['claude-usage', 'midjourney-stats', 'copilot-usage'],
  plugins: [
    {
      id: 'claude-usage',
      name: 'Claude Usage',
      description: 'Claude API 使用量和配额监控',
      author: 'CUK Official',
      version: '1.0.0',
      downloads: 25000,
      verified: true,
      icon: 'C',
      pluginType: 'data',
      dataType: 'usage',
      tags: ['official', 'usage', 'ai'],
      updatedAt: '2025-12-28T00:00:00Z',
    },
    {
      id: 'midjourney-stats',
      name: 'Midjourney 统计',
      description: '追踪剩余快速模式时长和生成次数',
      author: 'Community',
      version: '1.0.4',
      downloads: 12000,
      verified: true,
      icon: 'M',
      pluginType: 'data',
      dataType: 'usage',
      tags: ['ai', 'image', 'usage'],
      updatedAt: '2025-12-25T00:00:00Z',
    },
    {
      id: 'copilot-usage',
      name: 'Copilot 用量',
      description: '企业席位利用率监控和统计',
      author: 'CUK Official',
      version: '1.0.4',
      downloads: 8500,
      verified: true,
      icon: 'G',
      pluginType: 'data',
      dataType: 'usage',
      tags: ['official', 'usage', 'coding'],
      updatedAt: '2025-12-20T00:00:00Z',
    },
    {
      id: 'hf-status',
      name: 'HuggingFace 状态',
      description: '模型托管服务状态监控',
      author: 'Community',
      version: '1.0.4',
      downloads: 3200,
      verified: false,
      icon: 'H',
      pluginType: 'data',
      dataType: 'status',
      tags: ['ai', 'status', 'hosting'],
      updatedAt: '2025-12-15T00:00:00Z',
    },
    {
      id: 'openai-balance',
      name: 'OpenAI 余额',
      description: 'OpenAI API 余额和用量监控',
      author: 'Community',
      version: '1.2.0',
      downloads: 15000,
      verified: true,
      icon: 'O',
      pluginType: 'data',
      dataType: 'balance',
      tags: ['ai', 'balance', 'api'],
      updatedAt: '2025-12-22T00:00:00Z',
    },
    {
      id: 'cursor-usage',
      name: 'Cursor 用量',
      description: 'Cursor AI 编辑器使用量追踪',
      author: 'Community',
      version: '0.9.0',
      downloads: 4500,
      verified: false,
      icon: 'U',
      pluginType: 'data',
      dataType: 'usage',
      tags: ['ai', 'coding', 'usage'],
      updatedAt: '2025-12-18T00:00:00Z',
    },
    {
      id: 'replicate-status',
      name: 'Replicate 状态',
      description: 'Replicate 模型运行状态',
      author: 'Community',
      version: '1.0.0',
      downloads: 2100,
      verified: false,
      icon: 'R',
      pluginType: 'data',
      dataType: 'status',
      tags: ['ai', 'status', 'model'],
      updatedAt: '2025-12-10T00:00:00Z',
    },
    {
      id: 'anthropic-balance',
      name: 'Anthropic 余额',
      description: 'Anthropic API 余额查询',
      author: 'CUK Official',
      version: '1.0.0',
      downloads: 18000,
      verified: true,
      icon: 'A',
      pluginType: 'data',
      dataType: 'balance',
      tags: ['official', 'balance', 'api'],
      updatedAt: '2025-12-26T00:00:00Z',
    },
  ],
};

// 远程仓库 URL（可选，用于更新索引）
const REMOTE_REGISTRY_URL = 'https://raw.githubusercontent.com/cuk-team/cuk-plugins/main/registry.json';

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

  /**
   * 获取仓库索引
   * 优先使用缓存，超时后尝试刷新
   */
  async getRegistry(forceRefresh = false): Promise<PluginRegistry> {
    const now = Date.now();
    const cacheExpired = now - this.lastFetchTime > this.CACHE_TTL;

    if (!forceRefresh && !cacheExpired) {
      return this.registry;
    }

    // 尝试从远程获取（非阻塞）
    if (!this.isLoading) {
      this.fetchRemoteRegistry().catch(e => {
        console.warn('[Marketplace] 远程仓库获取失败，使用内置数据:', e);
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
      // 在 Tauri 环境中使用 fetch（需要配置 allowlist）
      // 在浏览器环境中直接使用 fetch
      const response = await fetch(REMOTE_REGISTRY_URL, {
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
      const result = await invoke<Result<PluginInfo>>('plugin_install', {
        source: plugin.downloadUrl || `registry://${pluginId}`,
        skipSignature: !plugin.verified,
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
