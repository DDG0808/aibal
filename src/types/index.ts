// 类型定义入口
// 从 contracts 目录引用类型定义

// 重新导出 contracts 中的所有类型
export type {
  // Plugin Context API
  LogLevel,
  PluginStorage,
  PluginCache,
  PluginContext,
  ExposedMethods,

  // Error Types
  PluginError,

  // IPC Commands
  Result,
  AppError,
  PluginType,
  DataType,
  PluginInfo,
  UpdateInfo,
  ValidationResult,
  HealthStatus,
  PluginHealth,
  PluginDataBase,
  UsageDimension,
  UsageData,
  BalanceData,
  StatusIndicator,
  StatusData,
  CustomData,
  PluginData,
  PluginManagementCommands,
  DataCommands,
  ConfigCommands,
  MonitoringCommands,
  IPCCommands,
  InvokeCommand,

  // IPC Events
  IPCEventName,
  PluginInstalledEvent,
  PluginUninstalledEvent,
  PluginUpdatedEvent,
  PluginDataUpdatedEvent,
  PluginErrorEvent,
  PluginHealthChangedEvent,
  IPCEvent,
  IPCEventHandlers,
  IPCEventListener,
  IPCEventPayload,

  // Plugin Module
  PluginMetadata,
  ConfigFieldSchema,
  ConfigValidationResult,
  DataPlugin,
  EventPlugin,
  HybridPlugin,
  PluginModule,
} from '@contracts/types';

// ============================================================================
// 运行时常量和函数（本地定义，不从 .d.ts 导入）
// ============================================================================

/**
 * 插件错误类型枚举
 * 与 contracts/types/errors.d.ts 保持一致
 */
export enum PluginErrorType {
  NETWORK_ERROR = 'NETWORK_ERROR',
  AUTH_ERROR = 'AUTH_ERROR',
  RATE_LIMIT = 'RATE_LIMIT',
  TIMEOUT = 'TIMEOUT',
  PARSE_ERROR = 'PARSE_ERROR',
  PROVIDER_ERROR = 'PROVIDER_ERROR',
  SANDBOX_LIMIT = 'SANDBOX_LIMIT',
  PERMISSION_DENIED = 'PERMISSION_DENIED',
  STORAGE_LIMIT = 'STORAGE_LIMIT',
  CACHE_ERROR = 'CACHE_ERROR',
  INCOMPATIBLE_API_VERSION = 'INCOMPATIBLE_API_VERSION',
  UNKNOWN = 'UNKNOWN',
}

/**
 * 可重试的错误类型列表
 */
export const RETRYABLE_ERRORS: readonly PluginErrorType[] = [
  PluginErrorType.NETWORK_ERROR,
  PluginErrorType.TIMEOUT,
  PluginErrorType.RATE_LIMIT,
  PluginErrorType.PROVIDER_ERROR,
  PluginErrorType.STORAGE_LIMIT,
  PluginErrorType.CACHE_ERROR,
] as const;

/**
 * 检查错误是否可重试
 */
export function isRetryable(errorType: PluginErrorType): boolean {
  return (RETRYABLE_ERRORS as readonly PluginErrorType[]).includes(errorType);
}

/**
 * 从 HTTP 状态码推断错误类型
 */
export function errorTypeFromHttpStatus(status: number): PluginErrorType {
  if (status === 401 || status === 403) return PluginErrorType.AUTH_ERROR;
  if (status === 429) return PluginErrorType.RATE_LIMIT;
  if (status === 408 || status === 504) return PluginErrorType.TIMEOUT;
  if (status >= 500 && status <= 599) return PluginErrorType.PROVIDER_ERROR;
  return PluginErrorType.UNKNOWN;
}

/**
 * 契约版本
 */
export const CONTRACT_VERSION = '1.1.0';

/**
 * API 版本
 */
export const API_VERSION = '1.0';

/**
 * 冻结日期
 */
export const FROZEN_DATE = '2025-12-27';

// ============================================================================
// 前端专用类型
// ============================================================================

/**
 * 应用设置
 */
export interface AppSettings {
  /** 启用的插件 ID 列表 */
  enabledPlugins: string[];
  /** 自动刷新间隔 (毫秒) */
  refreshInterval: number;
  /** 启动时自动刷新 */
  refreshOnLaunch: boolean;
  /** 开机自启动 */
  launchAtLogin: boolean;
  /** 显示在菜单栏 */
  showInMenuBar: boolean;
}

/**
 * 默认应用设置
 */
export const DEFAULT_APP_SETTINGS: AppSettings = {
  enabledPlugins: [],
  refreshInterval: 300000, // 5 分钟
  refreshOnLaunch: true,
  launchAtLogin: false,
  showInMenuBar: true,
};

/**
 * 主题类型
 */
export type Theme = 'light' | 'dark' | 'system';

/**
 * 窗口类型
 */
export type WindowType = 'main' | 'settings' | 'setup';

/**
 * 刷新状态
 */
export interface RefreshState {
  /** 是否正在刷新 */
  isRefreshing: boolean;
  /** 上次刷新时间 */
  lastRefreshTime: string | null;
  /** 刷新错误 */
  error: string | null;
}
