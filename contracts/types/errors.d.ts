/**
 * CUK Plugin Error Types
 *
 * @version 1.0.0
 * @frozen 2025-12-27
 * @status FROZEN
 *
 * 插件错误类型枚举，用于结构化错误处理。
 * 此枚举定义为冻结状态，任何变更需遵循 api-version-policy.md 规定的流程。
 */

/**
 * 插件错误类型枚举
 *
 * 按错误来源分类：
 * - 网络相关: NETWORK_ERROR, TIMEOUT, RATE_LIMIT
 * - 认证相关: AUTH_ERROR
 * - 解析相关: PARSE_ERROR
 * - 服务商相关: PROVIDER_ERROR
 * - 沙盒相关: SANDBOX_LIMIT
 * - 权限相关: PERMISSION_DENIED
 * - 存储相关: STORAGE_LIMIT, CACHE_ERROR
 * - 版本相关: INCOMPATIBLE_API_VERSION
 * - 其他: UNKNOWN
 */
export enum PluginErrorType {
  /**
   * 网络错误
   * 触发条件: fetch 请求失败、DNS 解析失败、连接超时
   * 可重试: 是
   */
  NETWORK_ERROR = 'NETWORK_ERROR',

  /**
   * 认证错误
   * 触发条件: API 返回 401/403、凭证过期或无效
   * 可重试: 否 (需用户更新凭证)
   */
  AUTH_ERROR = 'AUTH_ERROR',

  /**
   * 服务端限流
   * 触发条件: API 返回 429、X-RateLimit-Remaining: 0
   * 可重试: 是 (使用指数退避)
   */
  RATE_LIMIT = 'RATE_LIMIT',

  /**
   * 执行超时
   * 触发条件: 插件执行超过 context.timeout、fetch 超时
   * 可重试: 是
   */
  TIMEOUT = 'TIMEOUT',

  /**
   * 解析错误
   * 触发条件: JSON 解析失败、响应格式不符合预期
   * 可重试: 否 (API 响应格式变化需更新插件)
   */
  PARSE_ERROR = 'PARSE_ERROR',

  /**
   * 服务商错误
   * 触发条件: API 返回 5xx、服务暂时不可用
   * 可重试: 是 (使用指数退避)
   */
  PROVIDER_ERROR = 'PROVIDER_ERROR',

  /**
   * 沙盒限制
   * 触发条件: 内存超限、访问禁止的 API、网络请求被拦截
   * 可重试: 否
   */
  SANDBOX_LIMIT = 'SANDBOX_LIMIT',

  /**
   * 权限拒绝
   * 触发条件: context.call() 调用未授权的插件/方法
   * 可重试: 否 (需更新 manifest.json permissions)
   */
  PERMISSION_DENIED = 'PERMISSION_DENIED',

  /**
   * 存储限制
   * 触发条件: storage 超出大小限制、键数量超限
   * 可重试: 是 (清理后重试)
   */
  STORAGE_LIMIT = 'STORAGE_LIMIT',

  /**
   * 缓存错误
   * 触发条件: cache 操作失败
   * 可重试: 是
   */
  CACHE_ERROR = 'CACHE_ERROR',

  /**
   * API 版本不兼容
   * 触发条件: 插件 apiVersion 与运行时不兼容
   * 可重试: 否 (需升级插件或应用)
   */
  INCOMPATIBLE_API_VERSION = 'INCOMPATIBLE_API_VERSION',

  /**
   * 未知错误
   * 触发条件: 未分类的其他错误
   * 可重试: 视情况
   */
  UNKNOWN = 'UNKNOWN',
}

/**
 * 插件错误类
 *
 * 在插件中使用示例:
 * ```javascript
 * throw new PluginError(
 *   PluginErrorType.AUTH_ERROR,
 *   'Session Key 已过期',
 *   { statusCode: 401 }
 * );
 * ```
 */
export declare class PluginError extends Error {
  /**
   * 错误类型
   */
  readonly type: PluginErrorType;

  /**
   * 错误详情 (可选)
   */
  readonly details?: unknown;

  /**
   * 创建插件错误
   * @param type 错误类型
   * @param message 用户可读的错误消息
   * @param details 可选的错误详情 (用于调试)
   */
  constructor(type: PluginErrorType, message: string, details?: unknown);

  /**
   * 序列化为 JSON
   */
  toJSON(): {
    type: PluginErrorType;
    message: string;
    details?: unknown;
  };
}

/**
 * 错误重试策略映射
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
export function isRetryable(errorType: PluginErrorType): boolean;

/**
 * 从 HTTP 状态码推断错误类型
 */
export function errorTypeFromHttpStatus(status: number): PluginErrorType;
