/**
 * CUK Internal Commands Contract
 *
 * @version 1.0.0
 * @status ACTIVE
 *
 * 定义内部使用的 IPC 命令，这些命令不属于插件系统契约，
 * 但需要在前后端之间保持类型安全。
 *
 * 与 ipc-commands.d.ts 的区别：
 * - ipc-commands.d.ts: 插件系统相关的 18 个 commands（Phase 2+ 实现）
 * - internal-commands.d.ts: 应用核心功能的 commands（Phase 1 已实现）
 */

// ============================================================================
// 系统 Commands (2个)
// ============================================================================

/**
 * 系统命令接口
 */
export interface SystemCommands {
  /**
   * 获取应用版本号
   * @returns 版本号字符串 (如 "0.1.0")
   */
  get_version(): Promise<string>;

  /**
   * 健康检查
   * @returns true 表示应用运行正常
   */
  health_check(): Promise<boolean>;
}

// ============================================================================
// Keychain Commands (3个) - macOS Only
// ============================================================================

/**
 * Keychain 命令接口
 *
 * 使用 macOS Keychain 安全存储敏感数据（如 API 密钥）。
 * 在非 macOS 平台上这些命令会返回错误。
 */
export interface KeychainCommands {
  /**
   * 存储 Keychain 项
   * @param args.service - 服务名称（如 "com.cuk.app"）
   * @param args.key - 键名（如 "claude_session_key"）
   * @param args.value - 要存储的值
   * @throws 存储失败时返回错误信息
   */
  keychain_set(args: {
    service: string;
    key: string;
    value: string;
  }): Promise<void>;

  /**
   * 获取 Keychain 项
   * @param args.service - 服务名称
   * @param args.key - 键名
   * @returns 存储的值，如果不存在返回 null
   * @throws 读取失败时返回错误信息
   */
  keychain_get(args: {
    service: string;
    key: string;
  }): Promise<string | null>;

  /**
   * 删除 Keychain 项
   * @param args.service - 服务名称
   * @param args.key - 键名
   * @throws 删除失败时返回错误信息（不存在的项不会报错）
   */
  keychain_delete(args: {
    service: string;
    key: string;
  }): Promise<void>;
}

// ============================================================================
// 所有内部 Commands
// ============================================================================

/**
 * 所有内部 IPC Commands (5个)
 */
export interface InternalCommands extends SystemCommands, KeychainCommands {}

/**
 * 内部命令名称类型
 */
export type InternalCommandName = keyof InternalCommands;
