/**
 * CUK Plugin System Type Definitions
 *
 * @version 1.1.0
 * @frozen 2025-12-27
 * @updated 2025-12-27
 * @status FROZEN
 *
 * 统一导出所有类型定义，供前后端共享使用。
 *
 * @example
 * ```typescript
 * // 在 Vue 前端项目中
 * import type {
 *   PluginContext,
 *   PluginData,
 *   IPCCommands,
 *   PluginErrorType,
 *   PluginModule,
 * } from '@cuk/contracts';
 * ```
 */

// ============================================================================
// Plugin Context API
// ============================================================================

export {
  LogLevel,
  PluginStorage,
  PluginCache,
  PluginContext,
  ExposedMethods,
} from './plugin-context';

// ============================================================================
// Error Types
// ============================================================================

export {
  PluginErrorType,
  PluginError,
  RETRYABLE_ERRORS,
  isRetryable,
  errorTypeFromHttpStatus,
} from './errors';

// ============================================================================
// IPC Commands
// ============================================================================

export {
  // 基础类型
  Result,
  AppError,

  // 插件类型
  PluginType,
  DataType,
  PluginInfo,
  UpdateInfo,
  ValidationResult,
  HealthStatus,
  PluginHealth,

  // 插件数据类型
  PluginDataBase,
  UsageDimension,
  UsageData,
  BalanceData,
  StatusIndicator,
  StatusData,
  CustomData,
  PluginData,

  // Commands
  PluginManagementCommands,
  DataCommands,
  ConfigCommands,
  MonitoringCommands,
  IPCCommands,
  InvokeCommand,
} from './ipc-commands';

// ============================================================================
// IPC Events
// ============================================================================

export {
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
} from './ipc-events';

// ============================================================================
// Internal Commands (非插件系统的内部 API)
// ============================================================================

export {
  SystemCommands,
  KeychainCommands,
  InternalCommands,
  InternalCommandName,
} from './internal-commands';

// ============================================================================
// Plugin Module Contract
// ============================================================================

export {
  PluginType as ModulePluginType,
  DataType as ModuleDataType,
  PluginMetadata,
  ConfigFieldSchema,
  PluginDataBase as ModulePluginDataBase,
  UsageData as ModuleUsageData,
  BalanceData as ModuleBalanceData,
  StatusData as ModuleStatusData,
  CustomData as ModuleCustomData,
  PluginData as ModulePluginData,
  ConfigValidationResult,
  DataPlugin,
  EventPlugin,
  HybridPlugin,
  PluginModule,
} from './plugin-module';

// ============================================================================
// 版本信息
// ============================================================================

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

/**
 * 更新日期
 */
export const UPDATED_DATE = '2025-12-27';
