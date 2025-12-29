/**
 * CUK Plugin Context API
 *
 * @version 1.1.0
 * @frozen 2025-12-27
 * @updated 2025-12-27
 * @status FROZEN
 *
 * 插件运行时上下文，提供插件与宿主应用交互的所有 API。
 * 此接口定义为冻结状态，任何变更需遵循 api-version-policy.md 规定的流程。
 */

/**
 * 日志级别
 */
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

/**
 * 插件专属存储 API
 *
 * 键空间: 每个插件独立，键名格式为 `{pluginId}:{key}`
 * 并发语义: 单插件内串行执行，无竞态
 * 存储位置: ~/.config/cuk/storage/{pluginId}.json
 * 大小限制: 单插件最大 1MB
 */
export interface PluginStorage {
  /**
   * 获取存储值
   * @param key 键名，只能包含字母、数字、下划线和连字符
   * @returns 存储的值，不存在则返回 undefined
   */
  get(key: string): Promise<unknown>;

  /**
   * 设置存储值
   * @param key 键名，只能包含字母、数字、下划线和连字符
   * @param value 要存储的值，必须可 JSON 序列化
   * @throws 如果值无法序列化或超出存储限制
   */
  set(key: string, value: unknown): Promise<void>;

  /**
   * 删除存储值
   * @param key 键名
   * @returns true 如果键存在并被删除，false 如果键不存在
   */
  delete(key: string): Promise<boolean>;

  /**
   * 列出所有键
   * @returns 当前插件的所有存储键
   */
  keys(): Promise<string[]>;

  /**
   * 清空当前插件的所有存储
   */
  clear(): Promise<void>;
}

/**
 * 插件缓存 API
 *
 * 键空间: 每个插件独立，键名格式为 `cache:{pluginId}:{key}`
 * 并发语义: 线程安全，支持并发读写
 * 存储位置: 内存 (moka::future::Cache)
 * 过期策略: TTL (Time-To-Live) + TTI (Time-To-Idle)
 * 默认 TTL: 5 分钟 (300000ms)
 * 默认 TTI: 2 分钟 (120000ms)
 */
export interface PluginCache {
  /**
   * 获取缓存值
   * @param key 缓存键
   * @returns 缓存的值，不存在或已过期则返回 null
   */
  get(key: string): Promise<unknown | null>;

  /**
   * 设置缓存值
   * @param key 缓存键
   * @param value 要缓存的值，必须可 JSON 序列化
   * @param ttlMs 过期时间 (毫秒)，默认 300000 (5分钟)
   */
  set(key: string, value: unknown, ttlMs?: number): Promise<void>;

  /**
   * 删除缓存值
   * @param key 缓存键
   */
  delete(key: string): Promise<void>;

  /**
   * 检查缓存是否存在且未过期
   * @param key 缓存键
   */
  has(key: string): Promise<boolean>;
}

/**
 * 插件运行时上下文
 */
export interface PluginContext {
  /**
   * 当前插件 ID
   * @readonly
   */
  readonly pluginId: string;

  /**
   * 插件配置 (从用户设置读取)
   * secret 字段的值会被正常下发，但在 UI 中脱敏显示
   * @readonly
   */
  readonly config: Readonly<Record<string, unknown>>;

  /**
   * 执行超时时间 (毫秒)
   * 默认: 30000 (30秒)
   * @readonly
   */
  readonly timeout: number;

  /**
   * 当前运行时支持的 API 版本
   * 格式: "major.minor"
   * @readonly
   * @since 1.0
   */
  readonly runtimeApiVersion: string;

  /**
   * 检查指定能力是否可用
   * 用于版本兼容性检查，避免调用不存在的 API
   *
   * @param capability 能力名称，格式: "{namespace}.{method}"
   * @returns true 如果能力可用，false 如果不可用
   * @since 1.0
   *
   * @example
   * if (context.hasCapability('cache.has')) {
   *   const exists = await context.cache.has('key');
   * }
   */
  hasCapability(capability: string): boolean;

  /**
   * 插件专属持久化存储
   */
  readonly storage: PluginStorage;

  /**
   * 插件专属内存缓存
   */
  readonly cache: PluginCache;

  /**
   * 输出日志
   * @param level 日志级别
   * @param message 日志消息
   */
  log(level: LogLevel, message: string): void;

  /**
   * 发布事件到事件总线
   *
   * 事件命名规范:
   * - 插件事件: plugin:{pluginId}:{action}
   * - 系统事件: system:{action} (仅内部使用)
   *
   * @param event 事件名称 (自动添加 plugin:{pluginId}: 前缀)
   * @param data 事件数据，必须可 JSON 序列化
   *
   * @example
   * // 发布 "plugin:claude-usage:data_updated" 事件
   * context.emit('data_updated', { percentage: 75 });
   */
  emit(event: string, data?: unknown): void;

  /**
   * 跨插件方法调用
   *
   * 权限要求:
   * - 调用方必须在 manifest.json 的 permissions 中声明 "call:{targetPluginId}:{method}"
   * - 被调用方必须在 exposedMethods 中导出该方法
   *
   * 调用深度限制: 最大 3 层
   * 超时: 继承当前上下文的剩余超时时间
   *
   * @param pluginId 目标插件 ID
   * @param method 目标方法名
   * @param params 调用参数
   * @returns 目标方法的返回值
   * @throws PluginError 如果权限不足、目标不存在、或调用失败
   *
   * @example
   * // 调用 notifications 插件的 send 方法
   * await context.call('notifications', 'send', { title: '告警', body: '使用量超限' });
   */
  call(pluginId: string, method: string, params?: unknown): Promise<unknown>;
}

/**
 * 插件导出的方法集合
 * 供其他插件通过 context.call() 调用
 */
export interface ExposedMethods {
  [methodName: string]: (params: unknown, context: PluginContext) => Promise<unknown>;
}
