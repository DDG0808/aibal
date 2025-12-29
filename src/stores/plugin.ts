/**
 * 插件状态管理
 * Phase 8: 管理插件列表、数据和健康状态
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { PluginInfo, PluginData, PluginHealth, Result } from '@/types';

export const usePluginStore = defineStore('plugin', () => {
  // 状态
  const plugins = ref<PluginInfo[]>([]);
  const pluginData = ref<Map<string, PluginData>>(new Map());
  const pluginHealth = ref<Map<string, PluginHealth>>(new Map());
  const isLoading = ref(false);
  const isRefreshing = ref(false);
  const error = ref<string | null>(null);

  // 计算属性
  const enabledPlugins = computed(() => plugins.value.filter(p => p.enabled));
  const healthyPlugins = computed(() => plugins.value.filter(p => p.healthy));
  const totalCalls = computed(() => {
    let total = 0;
    pluginHealth.value.forEach(h => {
      total += Math.floor(h.successRate * 100); // 模拟调用次数
    });
    return total;
  });
  const systemHealthRate = computed(() => {
    const healths = Array.from(pluginHealth.value.values());
    if (healths.length === 0) return 100;
    const sum = healths.reduce((acc, h) => acc + h.successRate, 0);
    return Math.round((sum / healths.length) * 100);
  });

  // 获取插件列表
  async function fetchPlugins(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await invoke<Result<PluginInfo[]>>('plugin_list');
      if (result.success && result.data) {
        plugins.value = result.data;
      } else {
        error.value = result.error?.message ?? '获取插件列表失败';
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : '获取插件列表失败';
    } finally {
      isLoading.value = false;
    }
  }

  // 获取所有插件数据
  async function fetchAllData(force = false): Promise<void> {
    isRefreshing.value = true;
    try {
      const result = await invoke<Result<PluginData[]>>('refresh_all', { force });
      if (result.success && result.data) {
        const dataMap = new Map<string, PluginData>();
        result.data.forEach(d => dataMap.set(d.pluginId, d));
        pluginData.value = dataMap;
      }
    } catch (e) {
      console.error('Failed to fetch plugin data:', e);
    } finally {
      isRefreshing.value = false;
    }
  }

  // 获取所有健康状态
  async function fetchAllHealth(): Promise<void> {
    try {
      const result = await invoke<Result<PluginHealth[]>>('get_all_health');
      if (result.success && result.data) {
        const healthMap = new Map<string, PluginHealth>();
        result.data.forEach(h => healthMap.set(h.pluginId, h));
        pluginHealth.value = healthMap;
      }
    } catch (e) {
      console.error('Failed to fetch plugin health:', e);
    }
  }

  // 启用插件
  async function enablePlugin(id: string): Promise<boolean> {
    try {
      const result = await invoke<Result>('plugin_enable', { id });
      if (result.success) {
        const plugin = plugins.value.find(p => p.id === id);
        if (plugin) plugin.enabled = true;
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }

  // 禁用插件
  async function disablePlugin(id: string): Promise<boolean> {
    try {
      const result = await invoke<Result>('plugin_disable', { id });
      if (result.success) {
        const plugin = plugins.value.find(p => p.id === id);
        if (plugin) plugin.enabled = false;
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }

  // 刷新单个插件
  async function refreshPlugin(id: string, force = false): Promise<PluginData | null> {
    try {
      const result = await invoke<Result<PluginData>>('refresh_plugin', { id, force });
      if (result.success && result.data) {
        pluginData.value.set(id, result.data);
        return result.data;
      }
      return null;
    } catch {
      return null;
    }
  }

  // 获取插件配置
  async function getPluginConfig(id: string): Promise<Record<string, unknown> | null> {
    try {
      const result = await invoke<Result<Record<string, unknown>>>('get_plugin_config', { id });
      if (result.success && result.data) {
        return result.data;
      }
      return null;
    } catch {
      return null;
    }
  }

  // 保存插件配置
  async function savePluginConfig(id: string, config: Record<string, unknown>): Promise<boolean> {
    try {
      const result = await invoke<Result>('set_plugin_config', { id, config });
      return result.success;
    } catch {
      return false;
    }
  }

  // 初始化
  async function init(): Promise<void> {
    await Promise.all([
      fetchPlugins(),
      fetchAllData(),
      fetchAllHealth(),
    ]);
  }

  return {
    // 状态
    plugins,
    pluginData,
    pluginHealth,
    isLoading,
    isRefreshing,
    error,
    // 计算属性
    enabledPlugins,
    healthyPlugins,
    totalCalls,
    systemHealthRate,
    // 方法
    fetchPlugins,
    fetchAllData,
    fetchAllHealth,
    enablePlugin,
    disablePlugin,
    refreshPlugin,
    getPluginConfig,
    savePluginConfig,
    init,
  };
});
