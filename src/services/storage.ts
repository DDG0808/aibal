// StorageService - 数据持久化服务
// 封装 tauri-plugin-store 提供统一的存储接口

import { Store } from '@tauri-apps/plugin-store';
import type { AppSettings } from '@/types';
import { DEFAULT_APP_SETTINGS } from '@/types';

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
} as const;

// ============================================================================
// StorageService 类
// ============================================================================

/**
 * 存储服务
 * 提供类型安全的 CRUD 操作
 */
class StorageService {
  private store: Store | null = null;
  private storePromise: Promise<Store> | null = null;
  private readonly storePath = 'cuk-store.json';

  /**
   * 获取 Store 实例
   */
  private async getStore(): Promise<Store> {
    if (this.store) {
      return this.store;
    }

    if (!this.storePromise) {
      this.storePromise = Store.load(this.storePath).then((store) => {
        this.store = store;
        return store;
      });
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
  }

  /**
   * 删除值
   */
  async delete(key: string): Promise<boolean> {
    const store = await this.getStore();
    return store.delete(key);
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
