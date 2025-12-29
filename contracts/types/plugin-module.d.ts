/**
 * CUK Plugin Module Contract
 *
 * @version 1.0.0
 * @created 2025-12-27
 * @status FROZEN
 *
 * 定义插件入口文件必须导出的接口和类型。
 * 此契约基于 Codex 审核建议创建，补齐插件模块导出规范。
 */

import { PluginContext, ExposedMethods } from './plugin-context';

// ============================================================================
// 插件元数据类型
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
 * 插件元数据 (在入口文件中导出)
 *
 * 注意: 此元数据也可以在 manifest.json 中定义
 * 如果两处都有定义，manifest.json 优先
 */
export interface PluginMetadata {
  /** 插件 ID (必填) */
  id: string;

  /** 显示名称 (必填) */
  name: string;

  /** 版本号 (必填) */
  version: string;

  /** API 版本 (必填) */
  apiVersion: string;

  /** 插件类型 (必填) */
  pluginType: PluginType;

  /** 数据类型 (DataPlugin/HybridPlugin 必填) */
  dataType?: DataType;

  /** 作者 */
  author?: string;

  /** 描述 */
  description?: string;

  /** 主页 URL */
  homepage?: string;

  /** 图标文件名 */
  icon?: string;

  /** 刷新间隔 (毫秒) */
  refreshIntervalMs?: number;

  /** 权限声明 */
  permissions?: string[];

  /** 配置项 Schema */
  configSchema?: Record<string, ConfigFieldSchema>;
}

/**
 * 配置字段 Schema
 */
export interface ConfigFieldSchema {
  type: 'string' | 'number' | 'boolean' | 'select';
  required?: boolean;
  secret?: boolean;
  label?: string;
  description?: string;
  default?: string | number | boolean;
  min?: number;
  max?: number;
  options?: Array<{ value: string; label: string }>;
}

// ============================================================================
// 插件数据类型
// ============================================================================

/**
 * 插件数据基础接口
 */
export interface PluginDataBase {
  /** 最后更新时间 (ISO 8601) */
  lastUpdated: string;
}

/**
 * 使用量数据
 */
export interface UsageData extends PluginDataBase {
  dataType: 'usage';
  percentage: number;
  used: number;
  limit: number;
  unit: string;
  resetTime?: string;
  resetLabel?: string;
  dimensions?: Array<{
    id: string;
    label: string;
    percentage: number;
    used: number;
    limit: number;
    resetTime?: string;
  }>;
}

/**
 * 余额数据
 */
export interface BalanceData extends PluginDataBase {
  dataType: 'balance';
  balance: number;
  currency: string;
  quota?: number;
  usedQuota?: number;
  expiresAt?: string;
}

/**
 * 状态数据
 */
export interface StatusData extends PluginDataBase {
  dataType: 'status';
  indicator: 'none' | 'minor' | 'major' | 'critical' | 'unknown';
  description: string;
}

/**
 * 自定义数据
 */
export interface CustomData extends PluginDataBase {
  dataType: 'custom';
  renderHtml?: string;
  payload: Record<string, unknown>;
  title?: string;
  subtitle?: string;
}

/**
 * 插件数据联合类型
 */
export type PluginData = UsageData | BalanceData | StatusData | CustomData;

// ============================================================================
// 插件模块导出接口
// ============================================================================

/**
 * 配置验证结果
 */
export interface ConfigValidationResult {
  valid: boolean;
  message?: string;
}

/**
 * DataPlugin 接口
 * 必须实现 fetchData
 */
export interface DataPlugin {
  /** 元数据 (必须导出) */
  metadata: PluginMetadata;

  /**
   * 获取数据 (必须实现)
   * @param config 用户配置
   * @param context 运行时上下文
   */
  fetchData(
    config: Record<string, unknown>,
    context: PluginContext
  ): Promise<PluginData>;

  /** 插件加载时调用 */
  onLoad?(context: PluginContext): Promise<void>;

  /** 插件卸载时调用 */
  onUnload?(context: PluginContext): Promise<void>;

  /** 验证配置 */
  validateConfig?(config: Record<string, unknown>): Promise<ConfigValidationResult>;
}

/**
 * EventPlugin 接口
 * 必须实现 onEvent 和 subscribedEvents
 */
export interface EventPlugin {
  /** 元数据 (必须导出) */
  metadata: PluginMetadata;

  /** 订阅的事件列表 (必须导出) */
  subscribedEvents: string[];

  /**
   * 事件处理函数 (必须实现)
   * @param event 事件名称
   * @param data 事件数据
   * @param context 运行时上下文
   */
  onEvent(
    event: string,
    data: unknown,
    context: PluginContext
  ): Promise<void>;

  /** 暴露给其他插件调用的方法 */
  exposedMethods?: ExposedMethods;

  /** 插件加载时调用 */
  onLoad?(context: PluginContext): Promise<void>;

  /** 插件卸载时调用 */
  onUnload?(context: PluginContext): Promise<void>;
}

/**
 * HybridPlugin 接口
 * 必须同时实现 fetchData 和 onEvent
 */
export interface HybridPlugin extends DataPlugin, EventPlugin {
  /** 元数据 (必须导出) */
  metadata: PluginMetadata;
}

/**
 * 插件模块联合类型
 */
export type PluginModule = DataPlugin | EventPlugin | HybridPlugin;

// ============================================================================
// 插件导出示例
// ============================================================================

/**
 * @example DataPlugin 示例
 * ```javascript
 * // plugin.js
 * export const metadata = {
 *   id: 'claude-usage',
 *   name: 'Claude 使用量',
 *   version: '1.0.0',
 *   apiVersion: '1.0',
 *   pluginType: 'data',
 *   dataType: 'usage',
 * };
 *
 * export async function fetchData(config, context) {
 *   const response = await fetch('https://api.example.com/usage', {
 *     headers: { 'Authorization': `Bearer ${config.apiKey}` }
 *   });
 *   const data = await response.json();
 *   return {
 *     dataType: 'usage',
 *     percentage: data.percentage,
 *     used: data.used,
 *     limit: data.limit,
 *     unit: 'tokens',
 *     lastUpdated: new Date().toISOString(),
 *   };
 * }
 * ```
 *
 * @example EventPlugin 示例
 * ```javascript
 * // plugin.js
 * export const metadata = {
 *   id: 'notifications',
 *   name: '通知',
 *   version: '1.0.0',
 *   apiVersion: '1.0',
 *   pluginType: 'event',
 * };
 *
 * export const subscribedEvents = [
 *   'plugin:claude-usage:threshold_exceeded',
 * ];
 *
 * export async function onEvent(event, data, context) {
 *   if (event === 'plugin:claude-usage:threshold_exceeded') {
 *     // 发送系统通知
 *   }
 * }
 *
 * export const exposedMethods = {
 *   async send(params, context) {
 *     // 发送通知逻辑
 *   }
 * };
 * ```
 */
