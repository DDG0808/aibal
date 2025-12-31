// StorageService - 数据持久化服务
// 封装 tauri-plugin-store 提供统一的存储接口
// 支持浏览器 fallback（开发调试用）

import type { AppSettings } from '@/types';
import { DEFAULT_APP_SETTINGS } from '@/types';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Tauri Store 动态导入在 getStore() 中进行

// ============================================================================
// 存储键常量
// ============================================================================

export const STORAGE_KEYS = {
  /** 应用设置 */
  APP_SETTINGS: 'app_settings',
  /** 启用的插件列表 */
  ENABLED_PLUGINS: 'enabled_plugins',
  /** 插件配置 (前缀) */
  PLUGIN_CONFIG_PREFIX: 'plugin_config:',
  /** 插件数据缓存 (前缀) */
  PLUGIN_DATA_PREFIX: 'plugin_data:',
  /** 首次设置完成标志 */
  SETUP_COMPLETED: 'setup_completed',
  /** 仪表盘选中的插件 ID */
  SELECTED_PLUGIN_ID: 'selected_plugin_id',
} as const;

// ============================================================================
// StorageService 类
// ============================================================================

// 浏览器 localStorage fallback 接口
interface BrowserStore {
  get<T>(key: string): Promise<T | null>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<boolean>;
  has(key: string): Promise<boolean>;
  keys(): Promise<string[]>;
  clear(): Promise<void>;
  save(): Promise<void>;
}

// 浏览器 localStorage fallback 实现
const browserStore: BrowserStore = {
  async get<T>(key: string): Promise<T | null> {
    const value = localStorage.getItem(key);
    if (value === null) return null;
    try {
      return JSON.parse(value) as T;
    } catch {
      return null;
    }
  },
  async set(key: string, value: unknown): Promise<void> {
    localStorage.setItem(key, JSON.stringify(value));
  },
  async delete(key: string): Promise<boolean> {
    const exists = localStorage.getItem(key) !== null;
    localStorage.removeItem(key);
    return exists;
  },
  async has(key: string): Promise<boolean> {
    return localStorage.getItem(key) !== null;
  },
  async keys(): Promise<string[]> {
    return Object.keys(localStorage);
  },
  async clear(): Promise<void> {
    localStorage.clear();
  },
  async save(): Promise<void> {
    // localStorage 自动持久化，无需手动保存
  },
};

/**
 * 存储服务
 * 提供类型安全的 CRUD 操作
 * 支持 Tauri Store 和浏览器 localStorage fallback
 */
class StorageService {
  private store: BrowserStore | null = null;
  private storePromise: Promise<BrowserStore> | null = null;
  private readonly storePath = 'cuk-store.json';

  /**
   * 获取 Store 实例
   */
  private async getStore(): Promise<BrowserStore> {
    if (this.store) {
      return this.store;
    }

    if (!this.storePromise) {
      this.storePromise = (async () => {
        // 浏览器环境使用 localStorage fallback
        if (!isTauri) {
          console.info('[Storage] 使用浏览器 localStorage fallback');
          this.store = browserStore;
          return browserStore;
        }

        // Tauri 环境使用 tauri-plugin-store
        try {
          const { Store } = await import('@tauri-apps/plugin-store');
          const tauriStore = await Store.load(this.storePath);
          this.store = tauriStore as unknown as BrowserStore;
          return this.store;
        } catch (e) {
          console.warn('[Storage] Tauri Store 加载失败，使用 localStorage fallback:', e);
          this.store = browserStore;
          return browserStore;
        }
      })();
    }

    return this.storePromise;
  }

  /**
   * 获取值
   */
  async get<T>(key: string): Promise<T | null> {
    const store = await this.getStore();
    return (await store.get<T>(key)) ?? null;
  }

  /**
   * 设置值
   */
  async set<T>(key: string, value: T): Promise<void> {
    const store = await this.getStore();
    await store.set(key, value);
    // 持久化到磁盘，确保数据在重启后不丢失
    await store.save();
  }

  /**
   * 删除值
   */
  async delete(key: string): Promise<boolean> {
    const store = await this.getStore();
    const result = await store.delete(key);
    // 持久化到磁盘
    await store.save();
    return result;
  }

  /**
   * 检查键是否存在
   */
  async has(key: string): Promise<boolean> {
    const store = await this.getStore();
    return store.has(key);
  }

  /**
   * 获取所有键
   */
  async keys(): Promise<string[]> {
    const store = await this.getStore();
    return store.keys();
  }

  /**
   * 清空所有数据
   */
  async clear(): Promise<void> {
    const store = await this.getStore();
    await store.clear();
    // 持久化到磁盘
    await store.save();
  }

  /**
   * 保存到磁盘
   */
  async save(): Promise<void> {
    const store = await this.getStore();
    await store.save();
  }

  // ==========================================================================
  // 应用设置相关方法
  // ==========================================================================

  /**
   * 获取应用设置
   */
  async getAppSettings(): Promise<AppSettings> {
    const settings = await this.get<AppSettings>(STORAGE_KEYS.APP_SETTINGS);
    return settings ?? DEFAULT_APP_SETTINGS;
  }

  /**
   * 保存应用设置
   */
  async setAppSettings(settings: AppSettings): Promise<void> {
    await this.set(STORAGE_KEYS.APP_SETTINGS, settings);
  }

  /**
   * 更新应用设置（部分更新）
   */
  async updateAppSettings(updates: Partial<AppSettings>): Promise<AppSettings> {
    const current = await this.getAppSettings();
    const updated = { ...current, ...updates };
    await this.setAppSettings(updated);
    return updated;
  }

  // ==========================================================================
  // 插件配置相关方法
  // ==========================================================================

  /**
   * 获取插件配置
   */
  async getPluginConfig<T extends Record<string, unknown>>(pluginId: string): Promise<T | null> {
    return this.get<T>(`${STORAGE_KEYS.PLUGIN_CONFIG_PREFIX}${pluginId}`);
  }

  /**
   * 保存插件配置
   */
  async setPluginConfig<T extends Record<string, unknown>>(
    pluginId: string,
    config: T
  ): Promise<void> {
    await this.set(`${STORAGE_KEYS.PLUGIN_CONFIG_PREFIX}${pluginId}`, config);
  }

  /**
   * 删除插件配置
   */
  async deletePluginConfig(pluginId: string): Promise<boolean> {
    return this.delete(`${STORAGE_KEYS.PLUGIN_CONFIG_PREFIX}${pluginId}`);
  }

  // ==========================================================================
  // 首次设置相关方法
  // ==========================================================================

  /**
   * 检查是否完成首次设置
   */
  async hasCompletedSetup(): Promise<boolean> {
    return (await this.get<boolean>(STORAGE_KEYS.SETUP_COMPLETED)) ?? false;
  }

  /**
   * 标记首次设置完成
   */
  async markSetupCompleted(): Promise<void> {
    await this.set(STORAGE_KEYS.SETUP_COMPLETED, true);
  }
}

// 导出单例
export const storageService = new StorageService();
