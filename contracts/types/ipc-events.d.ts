/**
 * CUK IPC Events Contract
 *
 * @version 1.0.0
 * @frozen 2025-12-27
 * @status FROZEN
 *
 * 定义 Rust 后端向 Vue 前端推送的事件契约。
 * 共 6 个 IPC Events。
 */

import { PluginInfo, PluginData, PluginHealth, AppError } from './ipc-commands';

// ============================================================================
// IPC Events 定义
// ============================================================================

/**
 * IPC 事件名称
 * 格式: ipc:{action}
 */
export type IPCEventName =
  | 'ipc:plugin_installed'
  | 'ipc:plugin_uninstalled'
  | 'ipc:plugin_updated'
  | 'ipc:plugin_data_updated'
  | 'ipc:plugin_error'
  | 'ipc:plugin_health_changed';

/**
 * 插件安装完成事件
 */
export interface PluginInstalledEvent {
  /** 事件名称 */
  event: 'ipc:plugin_installed';
  /** 事件数据 */
  payload: PluginInfo;
}

/**
 * 插件卸载完成事件
 */
export interface PluginUninstalledEvent {
  /** 事件名称 */
  event: 'ipc:plugin_uninstalled';
  /** 事件数据 */
  payload: {
    /** 被卸载的插件 ID */
    id: string;
  };
}

/**
 * 插件更新完成事件
 */
export interface PluginUpdatedEvent {
  /** 事件名称 */
  event: 'ipc:plugin_updated';
  /** 事件数据 */
  payload: PluginInfo;
}

/**
 * 插件数据更新事件
 */
export interface PluginDataUpdatedEvent {
  /** 事件名称 */
  event: 'ipc:plugin_data_updated';
  /** 事件数据 */
  payload: {
    /** 插件 ID */
    id: string;
    /** 更新后的数据 */
    data: PluginData;
  };
}

/**
 * 插件错误事件
 */
export interface PluginErrorEvent {
  /** 事件名称 */
  event: 'ipc:plugin_error';
  /** 事件数据 */
  payload: {
    /** 插件 ID */
    id: string;
    /** 错误信息 */
    error: AppError;
  };
}

/**
 * 插件健康状态变化事件
 */
export interface PluginHealthChangedEvent {
  /** 事件名称 */
  event: 'ipc:plugin_health_changed';
  /** 事件数据 */
  payload: PluginHealth;
}

/**
 * 所有 IPC 事件联合类型
 */
export type IPCEvent =
  | PluginInstalledEvent
  | PluginUninstalledEvent
  | PluginUpdatedEvent
  | PluginDataUpdatedEvent
  | PluginErrorEvent
  | PluginHealthChangedEvent;

// ============================================================================
// 事件监听器类型
// ============================================================================

/**
 * 事件处理函数类型映射
 */
export interface IPCEventHandlers {
  'ipc:plugin_installed': (payload: PluginInstalledEvent['payload']) => void;
  'ipc:plugin_uninstalled': (payload: PluginUninstalledEvent['payload']) => void;
  'ipc:plugin_updated': (payload: PluginUpdatedEvent['payload']) => void;
  'ipc:plugin_data_updated': (payload: PluginDataUpdatedEvent['payload']) => void;
  'ipc:plugin_error': (payload: PluginErrorEvent['payload']) => void;
  'ipc:plugin_health_changed': (payload: PluginHealthChangedEvent['payload']) => void;
}

/**
 * Tauri 事件监听辅助类型
 *
 * @example
 * ```typescript
 * import { listen } from '@tauri-apps/api/event';
 *
 * // 监听插件数据更新
 * const unlisten = await listen<PluginDataUpdatedEvent['payload']>(
 *   'ipc:plugin_data_updated',
 *   (event) => {
 *     console.log('Plugin data updated:', event.payload.id, event.payload.data);
 *   }
 * );
 *
 * // 清理监听器
 * unlisten();
 * ```
 */
export type IPCEventListener<T extends IPCEventName> = IPCEventHandlers[T];

// ============================================================================
// 事件发射辅助 (后端使用)
// ============================================================================

/**
 * 获取事件的 Payload 类型
 *
 * @example
 * ```typescript
 * // 在 Rust 中发射事件时，确保 payload 类型正确
 * type Payload = IPCEventPayload<'ipc:plugin_data_updated'>;
 * // Payload = { id: string; data: PluginData }
 * ```
 */
export type IPCEventPayload<T extends IPCEventName> = Extract<
  IPCEvent,
  { event: T }
>['payload'];
