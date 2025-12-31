/**
 * 统一的 IPC 服务模块
 * 提供 Tauri IPC 调用的安全封装，支持浏览器环境的模拟数据
 */

import type {
  Result,
  PluginInfo,
  PluginData,
  PluginHealth,
} from '@/types';

// ============================================================================
// 环境检测
// ============================================================================

/**
 * 检测是否在 Tauri 环境中运行
 */
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// ============================================================================
// 模拟数据（浏览器开发调试用）
// ============================================================================

/**
 * 统一的模拟数据
 * 用于浏览器环境开发调试
 */
function getMockResult(cmd: string): unknown {
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
            author: 'AiBal Team',
            pluginType: 'data' as const,
            dataType: 'usage' as const,
            enabled: true,
            healthy: true,
          },
          {
            id: 'openai-api',
            name: 'OpenAI API',
            version: '1.0.0',
            description: 'OpenAI API 余额监控',
            author: 'AiBal Team',
            pluginType: 'data' as const,
            dataType: 'balance' as const,
            enabled: true,
            healthy: true,
          },
          {
            id: 'deepseek',
            name: 'DeepSeek',
            version: '1.0.0',
            description: 'DeepSeek API 余额监控',
            author: 'AiBal Team',
            pluginType: 'data' as const,
            dataType: 'balance' as const,
            enabled: true,
            healthy: true,
          },
        ] as PluginInfo[],
      } as Result<PluginInfo[]>;

    case 'get_all_data':
    case 'refresh_all':
      return {
        success: true,
        data: [
          {
            pluginId: 'claude-usage',
            dataType: 'usage' as const,
            percentage: 78,
            used: 780,
            limit: 1000,
            unit: 'msgs',
            resetTime: new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString(),
            resetLabel: '2小时15分后重置',
            lastUpdated: new Date().toISOString(),
            dimensions: [],
          },
          {
            pluginId: 'openai-api',
            dataType: 'balance' as const,
            balance: 12.45,
            currency: 'USD',
            lastUpdated: new Date().toISOString(),
          },
          {
            pluginId: 'deepseek',
            dataType: 'balance' as const,
            balance: 45.00,
            currency: 'CNY',
            lastUpdated: new Date().toISOString(),
          },
        ] as PluginData[],
      } as Result<PluginData[]>;

    case 'get_all_health':
      return {
        success: true,
        data: [
          {
            pluginId: 'claude-usage',
            status: 'healthy' as const,
            successRate: 0.99,
            avgLatencyMs: 150,
            errorCount: 0,
            lastSuccess: new Date().toISOString(),
          },
          {
            pluginId: 'openai-api',
            status: 'healthy' as const,
            successRate: 0.98,
            avgLatencyMs: 200,
            errorCount: 0,
            lastSuccess: new Date().toISOString(),
          },
          {
            pluginId: 'deepseek',
            status: 'healthy' as const,
            successRate: 0.97,
            avgLatencyMs: 180,
            errorCount: 0,
            lastSuccess: new Date().toISOString(),
          },
        ] as PluginHealth[],
      } as Result<PluginHealth[]>;

    case 'get_version':
      return '2.2 (Browser)';

    case 'plugin_enable':
    case 'plugin_disable':
    case 'plugin_reload':
    case 'plugin_uninstall':
      return { success: true } as Result<void>;

    case 'get_plugin_config':
      return { success: true, data: {} } as Result<Record<string, unknown>>;

    case 'set_plugin_config':
    case 'validate_plugin_config':
      return { success: true, data: { valid: true } };

    case 'plugin_check_updates':
      return { success: true, data: [] };

    default:
      console.warn(`[Mock] 未处理的命令: ${cmd}`);
      return { success: true, data: null };
  }
}

// ============================================================================
// IPC 调用函数
// ============================================================================

/**
 * 安全的 Tauri IPC 调用
 * 在浏览器环境返回模拟数据，在 Tauri 环境调用真实后端
 */
export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    console.info(`[Mock] invoke('${cmd}')`, args);
    return getMockResult(cmd) as T;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/**
 * 安全的事件监听
 * 在浏览器环境返回空清理函数
 */
export async function safeListen<T>(
  event: string,
  handler: (event: { payload: T }) => void
): Promise<() => void> {
  if (!isTauri) {
    console.info(`[Mock] listen('${event}')`);
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, handler);
}

/**
 * 安全的事件发送（仅当前窗口）
 * 在浏览器环境仅打印日志
 */
export async function safeEmit(event: string, payload?: unknown): Promise<void> {
  if (!isTauri) {
    console.info(`[Mock] emit('${event}')`, payload);
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const currentWindow = getCurrentWindow();
  await currentWindow.emit(event, payload);
}

/**
 * 安全的全局事件广播（所有窗口）
 * 在浏览器环境仅打印日志
 */
export async function safeEmitAll(event: string, payload?: unknown): Promise<void> {
  if (!isTauri) {
    console.info(`[Mock] emitAll('${event}')`, payload);
    return;
  }

  const { emit } = await import('@tauri-apps/api/event');
  await emit(event, payload);
}

// ============================================================================
// IPC 服务对象（可选的面向对象接口）
// ============================================================================

/**
 * IPC 服务单例
 * 提供统一的 IPC 调用接口
 */
export const ipcService = {
  isTauri,
  invoke: safeInvoke,
  listen: safeListen,
  emit: safeEmit,
  emitAll: safeEmitAll,
};
