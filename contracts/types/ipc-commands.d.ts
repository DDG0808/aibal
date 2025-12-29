/**
 * CUK IPC Commands Contract
 *
 * @version 1.1.0
 * @frozen 2025-12-27
 * @updated 2025-12-27
 * @status FROZEN
 *
 * 定义 Rust 后端与 Vue 前端之间的 IPC 通信契约。
 * 共 18 个 Commands，按功能分为 4 类。
 */

// 注意: PluginErrorType 在 errors.d.ts 中定义，此处不导入以避免循环依赖
// 如需使用，请单独导入: import { PluginErrorType } from './errors';

// ============================================================================
// 基础类型
// ============================================================================

/**
 * 操作结果包装器
 */
export interface Result<T = void> {
  success: boolean;
  data?: T;
  error?: AppError;
}

/**
 * 应用错误
 */
export interface AppError {
  /** 错误码 */
  code: string;
  /** 用户可读消息 */
  message: string;
  /** 详细信息 (用于调试) */
  details?: unknown;
}

// ============================================================================
// 插件相关类型
// ============================================================================

/**
 * 插件类型
 */
export type PluginType = 'data' | 'event' | 'hybrid';

/**
 * 数据类型
 */
export type DataType = 'usage' | 'balance' | 'status' | 'custom';

/**
 * 插件信息
 */
export interface PluginInfo {
  /** 插件 ID */
  id: string;
  /** 显示名称 */
  name: string;
  /** 版本号 */
  version: string;
  /** 插件类型 */
  pluginType: PluginType;
  /** 数据类型 (DataPlugin/HybridPlugin) */
  dataType?: DataType;
  /** 是否启用 */
  enabled: boolean;
  /** 是否健康 */
  healthy: boolean;
  /** 作者 */
  author?: string;
  /** 描述 */
  description?: string;
  /** 图标文件名 */
  icon?: string;
}

/**
 * 更新信息
 */
export interface UpdateInfo {
  /** 插件 ID */
  id: string;
  /** 当前版本 */
  currentVersion: string;
  /** 最新版本 */
  latestVersion: string;
  /** 更新说明 */
  releaseNotes?: string;
  /** 下载地址 */
  downloadUrl: string;
  /**
   * 插件包文件哈希
   * 格式: "sha256:{hex64}" (与 manifest.files 格式一致)
   * @example "sha256:abc123def456..."
   */
  sha256: string;
  /**
   * manifest.json 签名
   * 格式: "ed25519:{key_id}:{base64}" (与 manifest.signature 格式一致)
   * @example "ed25519:cuk-official-2025:MEUCIQDf..."
   */
  signature: string;
}

/**
 * 配置验证结果
 */
export interface ValidationResult {
  /** 是否有效 */
  valid: boolean;
  /** 错误消息 */
  message?: string;
  /** 字段级错误 */
  fieldErrors?: Record<string, string>;
}

/**
 * 健康状态
 */
export type HealthStatus = 'healthy' | 'degraded' | 'unhealthy';

/**
 * 插件健康信息
 */
export interface PluginHealth {
  /** 插件 ID */
  pluginId: string;
  /** 健康状态 */
  status: HealthStatus;
  /** 最后成功时间 (ISO 8601) */
  lastSuccess?: string;
  /** 最后错误信息 */
  lastError?: string;
  /** 错误计数 */
  errorCount: number;
  /** 平均延迟 (ms) */
  avgLatencyMs: number;
  /** 成功率 (0-1) */
  successRate: number;
}

// ============================================================================
// 插件数据类型
// ============================================================================

/**
 * 插件数据基础接口
 */
export interface PluginDataBase {
  /** 数据来源插件 */
  pluginId: string;
  /** 最后更新时间 (ISO 8601) */
  lastUpdated: string;
}

/**
 * Usage 维度
 */
export interface UsageDimension {
  /** 维度 ID */
  id: string;
  /** 显示标签 */
  label: string;
  /** 使用百分比 */
  percentage: number;
  /** 已用量 */
  used: number;
  /** 限额 */
  limit: number;
  /** 重置时间 */
  resetTime?: string;
}

/**
 * 使用量数据
 */
export interface UsageData extends PluginDataBase {
  dataType: 'usage';
  /** 使用百分比 (0-100) */
  percentage: number;
  /** 已用量 */
  used: number;
  /** 限额 */
  limit: number;
  /** 单位 */
  unit: string;
  /** 重置时间 (ISO 8601) */
  resetTime?: string;
  /** 重置标签 */
  resetLabel?: string;
  /** 多维度使用量 */
  dimensions?: UsageDimension[];
}

/**
 * 余额数据
 */
export interface BalanceData extends PluginDataBase {
  dataType: 'balance';
  /** 余额 */
  balance: number;
  /** 货币 */
  currency: string;
  /** 总额度 */
  quota?: number;
  /** 已用额度 */
  usedQuota?: number;
  /** 到期时间 (ISO 8601) */
  expiresAt?: string;
}

/**
 * 状态指示器
 */
export type StatusIndicator = 'none' | 'minor' | 'major' | 'critical' | 'unknown';

/**
 * 状态数据
 */
export interface StatusData extends PluginDataBase {
  dataType: 'status';
  /** 状态指示 */
  indicator: StatusIndicator;
  /** 状态描述 */
  description: string;
}

/**
 * 自定义数据
 */
export interface CustomData extends PluginDataBase {
  dataType: 'custom';
  /** 自定义渲染 HTML */
  renderHtml?: string;
  /** 自定义数据 */
  payload: Record<string, unknown>;
  /** 卡片标题 */
  title?: string;
  /** 卡片副标题 */
  subtitle?: string;
}

/**
 * 插件数据联合类型
 */
export type PluginData = UsageData | BalanceData | StatusData | CustomData;

// ============================================================================
// IPC Commands 定义
// ============================================================================

/**
 * 插件管理 Commands (9个)
 */
export interface PluginManagementCommands {
  /**
   * 获取所有插件列表
   */
  plugin_list(): Promise<Result<PluginInfo[]>>;

  /**
   * 启用插件
   */
  plugin_enable(args: { id: string }): Promise<Result>;

  /**
   * 禁用插件
   */
  plugin_disable(args: { id: string }): Promise<Result>;

  /**
   * 安装插件
   */
  plugin_install(args: {
    /** 插件来源 (URL 或本地路径) */
    source: string;
    /** 是否跳过签名验证 */
    skipSignature?: boolean;
  }): Promise<Result<PluginInfo>>;

  /**
   * 卸载插件
   */
  plugin_uninstall(args: { id: string }): Promise<Result>;

  /**
   * 重载插件
   */
  plugin_reload(args: { id: string }): Promise<Result>;

  /**
   * 检查插件更新
   */
  plugin_check_updates(): Promise<Result<UpdateInfo[]>>;

  /**
   * 更新插件
   */
  plugin_update(args: { id: string }): Promise<Result<PluginInfo>>;

  /**
   * 回滚插件
   */
  plugin_rollback(args: {
    id: string;
    /** 目标版本 */
    version: string;
  }): Promise<Result>;
}

/**
 * 数据 Commands (4个)
 */
export interface DataCommands {
  /**
   * 获取所有插件数据
   */
  get_all_data(): Promise<Result<PluginData[]>>;

  /**
   * 获取单个插件数据
   */
  get_plugin_data(args: { id: string }): Promise<Result<PluginData>>;

  /**
   * 刷新单个插件
   */
  refresh_plugin(args: {
    id: string;
    /** 是否强制刷新 (绕过缓存) */
    force?: boolean;
  }): Promise<Result<PluginData>>;

  /**
   * 刷新所有插件
   */
  refresh_all(args?: {
    /** 是否强制刷新 (绕过缓存) */
    force?: boolean;
  }): Promise<Result<PluginData[]>>;
}

/**
 * 配置 Commands (3个)
 */
export interface ConfigCommands {
  /**
   * 获取插件配置
   */
  get_plugin_config(args: { id: string }): Promise<Result<Record<string, unknown>>>;

  /**
   * 设置插件配置
   */
  set_plugin_config(args: {
    id: string;
    config: Record<string, unknown>;
  }): Promise<Result>;

  /**
   * 验证插件配置
   */
  validate_plugin_config(args: {
    id: string;
    config: Record<string, unknown>;
  }): Promise<Result<ValidationResult>>;
}

/**
 * 监控 Commands (2个)
 */
export interface MonitoringCommands {
  /**
   * 获取所有插件健康状态
   */
  get_all_health(): Promise<Result<PluginHealth[]>>;

  /**
   * 获取单个插件健康状态
   */
  get_plugin_health(args: { id: string }): Promise<Result<PluginHealth>>;
}

/**
 * 所有 IPC Commands (18个)
 */
export interface IPCCommands
  extends PluginManagementCommands,
    DataCommands,
    ConfigCommands,
    MonitoringCommands {}

// ============================================================================
// Tauri invoke 类型辅助
// ============================================================================

/**
 * 用于 Tauri invoke 的类型安全封装
 *
 * @example
 * ```typescript
 * import { invoke } from '@tauri-apps/api/core';
 *
 * const result = await invoke<Result<PluginInfo[]>>('plugin_list');
 * if (result.success) {
 *   console.log(result.data);
 * }
 * ```
 */
export type InvokeCommand = keyof IPCCommands;
