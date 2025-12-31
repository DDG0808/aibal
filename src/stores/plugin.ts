/**
 * 插件状态管理
 * Phase 8: 管理插件列表、数据和健康状态
 * Phase 8.2: 插件市场安装功能
 * 支持浏览器 fallback（开发调试用）
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { PluginInfo, PluginData, PluginHealth, Result, InstallStatus, UpdateInfo } from '@/types';
import { marketplaceService } from '@/services';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 安全的 invoke 调用（浏览器环境返回模拟数据）
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    console.info(`[Mock] invoke('${cmd}')`, args);
    // 返回模拟数据用于浏览器调试
    return getMockResult(cmd) as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// 模拟数据（浏览器开发调试用）
function getMockResult(cmd: string): Result<unknown> {
  switch (cmd) {
    case 'plugin_list':
      return {
        success: true,
        data: [
          {
            id: 'claude-usage',
            name: 'Claude Usage',
            version: '1.0.0',
            description: 'Claude API 使用量监控',
            author: 'CUK Team',
            pluginType: 'data' as const,
            dataType: 'usage' as const,
            enabled: true,
            healthy: true,
          },
        ],
      };
    case 'refresh_all':
    case 'get_all_data':
      return {
        success: true,
        data: [
          {
            pluginId: 'claude-usage',
            lastUpdated: new Date().toISOString(),
            dataType: 'usage' as const,
            percentage: 42,
            used: 420,
            limit: 1000,
            unit: 'msgs',
            resetTime: new Date(Date.now() + 3600000).toISOString(),
            resetLabel: '1h 后重置',
            dimensions: [],
          },
        ],
      };
    case 'get_all_health':
      return {
        success: true,
        data: [
          {
            pluginId: 'claude-usage',
            status: 'healthy' as const,
            successRate: 0.99,
            lastCheck: new Date().toISOString(),
          },
        ],
      };
    default:
      return { success: true, data: null };
  }
}

export const usePluginStore = defineStore('plugin', () => {
  // 状态
  const plugins = ref<PluginInfo[]>([]);
  const pluginData = ref<Map<string, PluginData>>(new Map());
  const pluginHealth = ref<Map<string, PluginHealth>>(new Map());
  const isLoading = ref(false);
  const isRefreshing = ref(false);
  const error = ref<string | null>(null);

  // 安装状态追踪 (pluginId -> InstallStatus)
  const installingPlugins = ref<Map<string, InstallStatus>>(new Map());
  const installErrors = ref<Map<string, string>>(new Map());

  // 操作中的插件 (防止竞态)
  const operatingPlugins = ref<Set<string>>(new Set());

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
      const result = await safeInvoke<Result<PluginInfo[]>>('plugin_list');
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
      const result = await safeInvoke<Result<PluginData[]>>('refresh_all', { force });
      if (result.success && result.data) {
        const dataMap = new Map<string, PluginData>();
        result.data.forEach(d => dataMap.set(d.pluginId, d));
        pluginData.value = dataMap;
      } else if (!result.success) {
        error.value = result.error?.message ?? '获取插件数据失败';
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : '获取插件数据失败';
    } finally {
      isRefreshing.value = false;
    }
  }

  // 获取所有健康状态
  async function fetchAllHealth(): Promise<void> {
    try {
      const result = await safeInvoke<Result<PluginHealth[]>>('get_all_health');
      if (result.success && result.data) {
        const healthMap = new Map<string, PluginHealth>();
        result.data.forEach(h => healthMap.set(h.pluginId, h));
        pluginHealth.value = healthMap;
      } else if (!result.success) {
        error.value = result.error?.message ?? '获取健康状态失败';
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : '获取健康状态失败';
    }
  }

  // 检查插件是否正在操作中
  function isOperating(id: string): boolean {
    return operatingPlugins.value.has(id);
  }

  // 启用插件
  async function enablePlugin(id: string): Promise<boolean> {
    if (operatingPlugins.value.has(id)) return false;
    operatingPlugins.value.add(id);
    try {
      const result = await safeInvoke<Result>('plugin_enable', { id });
      if (result.success) {
        const plugin = plugins.value.find(p => p.id === id);
        if (plugin) plugin.enabled = true;
        return true;
      }
      error.value = result.error?.message ?? '启用插件失败';
      return false;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '启用插件失败';
      return false;
    } finally {
      operatingPlugins.value.delete(id);
    }
  }

  // 禁用插件
  async function disablePlugin(id: string): Promise<boolean> {
    if (operatingPlugins.value.has(id)) return false;
    operatingPlugins.value.add(id);
    try {
      const result = await safeInvoke<Result>('plugin_disable', { id });
      if (result.success) {
        const plugin = plugins.value.find(p => p.id === id);
        if (plugin) plugin.enabled = false;
        return true;
      }
      error.value = result.error?.message ?? '禁用插件失败';
      return false;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '禁用插件失败';
      return false;
    } finally {
      operatingPlugins.value.delete(id);
    }
  }

  // 刷新单个插件
  async function refreshPlugin(id: string, force = false): Promise<PluginData | null> {
    try {
      const result = await safeInvoke<Result<PluginData>>('refresh_plugin', { id, force });
      if (result.success && result.data) {
        pluginData.value.set(id, result.data);
        return result.data;
      }
      if (!result.success) {
        error.value = result.error?.message ?? '刷新插件数据失败';
      }
      return null;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '刷新插件数据失败';
      return null;
    }
  }

  // 获取插件配置
  async function getPluginConfig(id: string): Promise<Record<string, unknown> | null> {
    try {
      const result = await safeInvoke<Result<Record<string, unknown>>>('get_plugin_config', { id });
      if (result.success && result.data) {
        return result.data;
      }
      if (!result.success) {
        error.value = result.error?.message ?? '获取插件配置失败';
      }
      return null;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '获取插件配置失败';
      return null;
    }
  }

  // 验证插件配置（返回结构与 contracts/types/ipc-commands.d.ts:ValidationResult 一致）
  async function validatePluginConfig(id: string, config: Record<string, unknown>): Promise<{
    valid: boolean;
    message?: string;
    fieldErrors?: Record<string, string>;
  }> {
    try {
      const result = await safeInvoke<Result<{
        valid: boolean;
        message?: string;
        field_errors?: Array<{ field: string; message: string; error_type: string }>;
      }>>('validate_plugin_config', { id, config });
      if (result.success && result.data) {
        // 将数组格式转换为 Record 格式（契约定义）
        const fieldErrors: Record<string, string> = {};
        if (result.data.field_errors) {
          for (const e of result.data.field_errors) {
            fieldErrors[e.field] = e.message;
          }
        }
        return {
          valid: result.data.valid,
          message: result.data.message,
          fieldErrors: Object.keys(fieldErrors).length > 0 ? fieldErrors : undefined,
        };
      }
      return { valid: false, message: result.error?.message ?? '验证失败' };
    } catch (e) {
      return { valid: false, message: e instanceof Error ? e.message : '验证失败' };
    }
  }

  // 保存插件配置
  async function savePluginConfig(id: string, config: Record<string, unknown>): Promise<boolean> {
    try {
      const result = await safeInvoke<Result>('set_plugin_config', { id, config });
      if (!result.success) {
        error.value = result.error?.message ?? '保存插件配置失败';
      }
      return result.success;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '保存插件配置失败';
      return false;
    }
  }

  // 卸载插件
  async function uninstallPlugin(id: string): Promise<boolean> {
    if (operatingPlugins.value.has(id)) return false;
    operatingPlugins.value.add(id);
    try {
      const result = await safeInvoke<Result>('plugin_uninstall', { id });
      if (result.success) {
        // 从本地列表移除
        plugins.value = plugins.value.filter(p => p.id !== id);
        pluginData.value.delete(id);
        pluginHealth.value.delete(id);
        return true;
      }
      error.value = result.error?.message ?? '卸载插件失败';
      return false;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '卸载插件失败';
      return false;
    } finally {
      operatingPlugins.value.delete(id);
    }
  }

  // 重载插件
  async function reloadPlugin(id: string): Promise<boolean> {
    if (operatingPlugins.value.has(id)) return false;
    operatingPlugins.value.add(id);
    try {
      const result = await safeInvoke<Result>('plugin_reload', { id });
      if (result.success) {
        // 重新获取插件信息和健康数据
        await Promise.all([
          fetchPlugins(),
          fetchAllData(),
          fetchAllHealth(),
        ]);
        return true;
      }
      error.value = result.error?.message ?? '重载插件失败';
      return false;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '重载插件失败';
      return false;
    } finally {
      operatingPlugins.value.delete(id);
    }
  }

  // 检查插件更新（返回类型与契约 UpdateInfo[] 对齐）
  async function checkUpdates(): Promise<UpdateInfo[]> {
    try {
      const result = await safeInvoke<Result<UpdateInfo[]>>('plugin_check_updates');
      if (result.success && result.data) {
        return result.data;
      }
      return [];
    } catch {
      return [];
    }
  }

  // 更新插件
  async function updatePlugin(id: string): Promise<boolean> {
    if (operatingPlugins.value.has(id)) return false;
    operatingPlugins.value.add(id);
    try {
      const result = await safeInvoke<Result<PluginInfo>>('plugin_update', { id });
      if (result.success && result.data) {
        // 更新本地插件信息
        const index = plugins.value.findIndex(p => p.id === id);
        if (index !== -1) {
          plugins.value[index] = result.data;
        }
        return true;
      }
      error.value = result.error?.message ?? '更新插件失败';
      return false;
    } catch (e) {
      error.value = e instanceof Error ? e.message : '更新插件失败';
      return false;
    } finally {
      operatingPlugins.value.delete(id);
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

  // ============================================================================
  // 插件市场安装功能 (Phase 8.2)
  // ============================================================================

  /**
   * 检查插件是否已安装
   */
  function isInstalled(pluginId: string): boolean {
    return plugins.value.some(p => p.id === pluginId);
  }

  /**
   * 获取插件安装状态
   */
  function getInstallStatus(pluginId: string): InstallStatus {
    return installingPlugins.value.get(pluginId) ?? 'idle';
  }

  /**
   * 获取安装错误信息
   */
  function getInstallError(pluginId: string): string | undefined {
    return installErrors.value.get(pluginId);
  }

  // 清理定时器追踪（修复 setTimeout 竞态问题）
  const cleanupTimers = ref<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // 需要签名确认的插件 (pluginId -> true)
  const pendingSignatureConfirm = ref<Map<string, boolean>>(new Map());

  /**
   * 从市场安装插件
   * 使用返回的 PluginInfo 直接更新本地列表（优化：避免额外 fetchPlugins）
   *
   * @param pluginId 插件 ID
   * @param skipSignature 是否跳过签名验证（用户确认后传 true）
   * @returns 'success' | 'need_confirm' | 'error'
   */
  async function installMarketplacePlugin(pluginId: string, skipSignature = false): Promise<'success' | 'need_confirm' | 'error'> {
    // 已安装检查
    if (isInstalled(pluginId)) {
      return 'success';
    }

    // 正在安装检查
    const currentStatus = installingPlugins.value.get(pluginId);
    if (currentStatus === 'downloading' || currentStatus === 'installing') {
      return 'error';
    }

    // 清除之前的错误和定时器
    installErrors.value.delete(pluginId);
    const existingTimer = cleanupTimers.value.get(pluginId);
    if (existingTimer) {
      clearTimeout(existingTimer);
      cleanupTimers.value.delete(pluginId);
    }

    try {
      // 设置下载状态
      installingPlugins.value.set(pluginId, 'downloading');

      // 调用市场服务安装
      const result = await marketplaceService.installPlugin(pluginId, skipSignature);

      if (result.success && result.data) {
        // 设置安装中状态
        installingPlugins.value.set(pluginId, 'installing');

        // 使用返回的 PluginInfo 直接添加到本地列表（避免额外 fetchPlugins）
        const newPlugin = result.data;
        if (!plugins.value.some(p => p.id === newPlugin.id)) {
          plugins.value.push(newPlugin);
        }

        // 设置成功状态
        installingPlugins.value.set(pluginId, 'success');
        pendingSignatureConfirm.value.delete(pluginId);

        // 异步刷新新插件的 data 和 health（非阻塞）
        // 新安装插件可能尚未产生数据，获取失败是正常的，静默处理
        Promise.all([fetchAllData(), fetchAllHealth()]).catch(() => {
          // 静默失败，后续用户操作会触发 init() 全量刷新兜底
        });

        // 3秒后清除状态（可追踪/可取消）
        const timer = setTimeout(() => {
          installingPlugins.value.delete(pluginId);
          cleanupTimers.value.delete(pluginId);
        }, 3000);
        cleanupTimers.value.set(pluginId, timer);

        return 'success';
      } else {
        // 检查是否是签名验证失败
        if (marketplaceService.isSignatureError(result.error?.code)) {
          // 需要用户确认
          installingPlugins.value.set(pluginId, 'idle');
          pendingSignatureConfirm.value.set(pluginId, true);
          return 'need_confirm';
        }

        // 其他错误
        installingPlugins.value.set(pluginId, 'error');
        installErrors.value.set(pluginId, result.error?.message ?? '安装失败');
        return 'error';
      }
    } catch (e) {
      // 设置错误状态
      installingPlugins.value.set(pluginId, 'error');
      const errorMsg = e instanceof Error ? e.message : '安装失败';
      installErrors.value.set(pluginId, errorMsg);
      return 'error';
    }
  }

  /**
   * 检查插件是否需要签名确认
   */
  function needsSignatureConfirm(pluginId: string): boolean {
    return pendingSignatureConfirm.value.get(pluginId) ?? false;
  }

  /**
   * 取消签名确认
   */
  function cancelSignatureConfirm(pluginId: string): void {
    pendingSignatureConfirm.value.delete(pluginId);
  }

  /**
   * 重置安装状态（同时清理定时器）
   */
  function resetInstallStatus(pluginId: string): void {
    installingPlugins.value.delete(pluginId);
    installErrors.value.delete(pluginId);
    // 清理定时器
    const timer = cleanupTimers.value.get(pluginId);
    if (timer) {
      clearTimeout(timer);
      cleanupTimers.value.delete(pluginId);
    }
  }

  return {
    // 状态
    plugins,
    pluginData,
    pluginHealth,
    isLoading,
    isRefreshing,
    error,
    // 安装状态
    installingPlugins,
    installErrors,
    // 操作状态
    operatingPlugins,
    // 计算属性
    enabledPlugins,
    healthyPlugins,
    totalCalls,
    systemHealthRate,
    // 方法
    fetchPlugins,
    fetchAllData,
    fetchAllHealth,
    isOperating,
    enablePlugin,
    disablePlugin,
    refreshPlugin,
    getPluginConfig,
    validatePluginConfig,
    savePluginConfig,
    uninstallPlugin,
    reloadPlugin,
    checkUpdates,
    updatePlugin,
    init,
    // 安装方法
    isInstalled,
    getInstallStatus,
    getInstallError,
    installMarketplacePlugin,
    resetInstallStatus,
    // 签名确认
    needsSignatureConfirm,
    cancelSignatureConfirm,
  };
});
