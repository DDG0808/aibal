// KeychainService - 敏感数据存储服务
// 使用 Tauri IPC 调用 Rust 后端的 Keychain 功能

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Keychain 服务名称
// ============================================================================

const KEYCHAIN_SERVICE = 'com.cuk.app';

// ============================================================================
// KeychainService 类
// ============================================================================

/**
 * Keychain 服务
 * 用于安全存储敏感数据（如 Session Key）
 */
class KeychainService {
  /**
   * 存储密钥
   * @param key 密钥名称
   * @param value 密钥值
   */
  async set(key: string, value: string): Promise<void> {
    await invoke('keychain_set', {
      service: KEYCHAIN_SERVICE,
      key,
      value,
    });
  }

  /**
   * 获取密钥
   * @param key 密钥名称
   * @returns 密钥值，不存在返回 null
   */
  async get(key: string): Promise<string | null> {
    try {
      const result = await invoke<string | null>('keychain_get', {
        service: KEYCHAIN_SERVICE,
        key,
      });
      return result;
    } catch {
      return null;
    }
  }

  /**
   * 删除密钥
   * @param key 密钥名称
   */
  async delete(key: string): Promise<void> {
    await invoke('keychain_delete', {
      service: KEYCHAIN_SERVICE,
      key,
    });
  }

  /**
   * 检查密钥是否存在
   * @param key 密钥名称
   */
  async has(key: string): Promise<boolean> {
    const value = await this.get(key);
    return value !== null;
  }
}

// ============================================================================
// 常用密钥名称
// ============================================================================

export const KEYCHAIN_KEYS = {
  /** Claude Session Key */
  CLAUDE_SESSION_KEY: 'claude_session_key',
  /** OpenAI API Key */
  OPENAI_API_KEY: 'openai_api_key',
} as const;

// 导出单例
export const keychainService = new KeychainService();
