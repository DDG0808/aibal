# AiBal 插件开发指南

> 版本: 1.0.0 | API 版本: 1.0

## 目录

- [概述](#概述)
- [快速开始](#快速开始)
- [插件结构](#插件结构)
- [插件类型](#插件类型)
- [Manifest 配置](#manifest-配置)
- [插件 API](#插件-api)
- [数据类型](#数据类型)
- [事件系统](#事件系统)
- [跨插件通信](#跨插件通信)
- [配置 Schema](#配置-schema)
- [安全与权限](#安全与权限)
- [调试与测试](#调试与测试)
- [发布指南](#发布指南)
- [完整示例](#完整示例)
- [常见问题](#常见问题)

---

## 概述

AiBal 插件系统采用三层架构设计，提供安全、可扩展的插件运行环境：

```
┌─────────────────────────────────────────┐
│        Vue Frontend (浏览器)             │
│  - 插件UI组件                            │
│  - Marketplace 市场                      │
└────────────┬───────────────────────────-─┘
             │ IPC Commands
             ▼
┌─────────────────────────────────────────┐
│    Tauri Backend (Rust)                  │
│  - PluginManager (生命周期管理)           │
│  - PluginExecutor (沙箱运行时)            │
│  - EventBus (事件总线)                   │
└────────────┬───────────────────────────-─┘
             │ QuickJS Sandbox
             ▼
┌─────────────────────────────────────────┐
│      Plugin Runtime (JavaScript)         │
│  - 沙箱执行环境                          │
│  - 安全 API 注入                         │
└─────────────────────────────────────────┘
```

### 核心特性

- **沙箱隔离**: QuickJS 沙箱环境，内存限制 16MB，执行超时 30s
- **权限模型**: 细粒度权限控制，跨插件调用需显式声明
- **事件驱动**: 插件间通过事件总线通信
- **类型安全**: 完整 TypeScript 类型定义
- **热重载**: 支持不重启应用更新插件

---

## 快速开始

### 1. 创建插件目录

```bash
mkdir my-plugin
cd my-plugin
```

### 2. 创建 manifest.json

```json
{
  "id": "my-plugin",
  "name": "我的第一个插件",
  "version": "1.0.0",
  "apiVersion": "1.0",
  "pluginType": "data",
  "dataType": "usage",
  "author": "Your Name",
  "description": "插件描述",
  "entry": "plugin.js",
  "refreshIntervalMs": 60000,
  "permissions": ["network"],
  "configSchema": {
    "apiKey": {
      "type": "string",
      "required": true,
      "secret": true,
      "label": "API Key"
    }
  }
}
```

### 3. 创建 plugin.js

```javascript
export const metadata = {
  id: 'my-plugin',
  name: '我的第一个插件',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'
};

export async function fetchData(config, context) {
  return {
    dataType: 'usage',
    percentage: 50,
    used: 500,
    limit: 1000,
    unit: 'tokens',
    lastUpdated: new Date().toISOString()
  };
}
```

### 4. 安装插件

将插件目录复制到 `~/.config/aibal/plugins/` 或通过应用内 Marketplace 安装。

---

## 插件结构

### 目录结构

```
my-plugin/
├── manifest.json      # 必需：插件清单文件
├── plugin.js          # 必需：入口文件
├── icon.png           # 可选：插件图标 (推荐 64x64)
└── assets/            # 可选：其他资源文件
```

### 入口文件导出

根据插件类型，入口文件需要导出不同的函数：

| 导出项 | DataPlugin | EventPlugin | HybridPlugin |
|--------|------------|-------------|--------------|
| `metadata` | ✅ 必需 | ✅ 必需 | ✅ 必需 |
| `fetchData` | ✅ 必需 | ❌ | ✅ 必需 |
| `onEvent` | ❌ | ✅ 必需 | ✅ 必需 |
| `subscribedEvents` | ❌ | ✅ 必需 | ✅ 必需 |
| `exposedMethods` | ❌ | ⭕ 可选 | ⭕ 可选 |
| `onLoad` | ⭕ 可选 | ⭕ 可选 | ⭕ 可选 |
| `onUnload` | ⭕ 可选 | ⭕ 可选 | ⭕ 可选 |
| `validateConfig` | ⭕ 可选 | ⭕ 可选 | ⭕ 可选 |

---

## 插件类型

### DataPlugin (数据插件)

用于获取和返回数据，如 API 使用量、账户余额、服务状态等。

```javascript
export const metadata = {
  id: 'usage-monitor',
  name: 'Usage Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'  // usage | balance | status | custom
};

export async function fetchData(config, context) {
  // 获取数据逻辑
  const response = await fetch('https://api.example.com/usage', {
    headers: { 'Authorization': `Bearer ${config.apiKey}` }
  });

  const data = await response.json();

  return {
    dataType: 'usage',
    percentage: data.percent,
    used: data.used,
    limit: data.limit,
    unit: 'tokens',
    lastUpdated: new Date().toISOString()
  };
}
```

### EventPlugin (事件插件)

用于监听和响应系统或其他插件发出的事件。

```javascript
export const metadata = {
  id: 'notifications',
  name: 'Notifications',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'event'
};

// 订阅的事件列表
export const subscribedEvents = [
  'plugin:usage-monitor:threshold_exceeded',
  'system:app_ready'
];

// 暴露给其他插件调用的方法
export const exposedMethods = ['send', 'queue', 'clear'];

export async function onEvent(event, data, context) {
  if (event === 'plugin:usage-monitor:threshold_exceeded') {
    await send({
      title: '使用量警告',
      message: `使用量已达 ${data.percentage}%`
    });
  }
}

// 暴露的方法实现
async function send(params) {
  console.log('Notification:', params.title, params.message);
  return { success: true };
}
```

### HybridPlugin (混合插件)

同时具备数据获取和事件响应能力。

```javascript
export const metadata = {
  id: 'smart-monitor',
  name: 'Smart Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'hybrid',
  dataType: 'usage'
};

export const subscribedEvents = [
  'system:refresh_requested'
];

export async function fetchData(config, context) {
  // 返回数据
  return { /* ... */ };
}

export async function onEvent(event, data, context) {
  // 处理事件
}
```

---

## Manifest 配置

### 完整字段说明

```json
{
  // ========== 必需字段 ==========
  "id": "plugin-id",           // 唯一标识符，小写字母、数字、连字符
  "name": "Plugin Name",       // 显示名称
  "version": "1.0.0",          // 语义化版本号
  "apiVersion": "1.0",         // API 版本 (当前支持 "1.0")
  "pluginType": "data",        // 插件类型: data | event | hybrid

  // ========== 数据插件必需 ==========
  "dataType": "usage",         // 数据类型: usage | balance | status | custom

  // ========== 可选字段 ==========
  "author": "Author Name",     // 作者
  "description": "描述文字",    // 插件描述
  "homepage": "https://...",   // 主页链接
  "icon": "icon.png",          // 图标文件路径
  "entry": "plugin.js",        // 入口文件 (默认: plugin.js)
  "refreshIntervalMs": 60000,  // 数据刷新间隔 (毫秒)

  // ========== 权限声明 ==========
  "permissions": [
    "network",                  // 网络请求
    "storage",                  // 持久化存储
    "cache",                    // 内存缓存
    "timer",                    // setTimeout/setInterval
    "call:notifications:send"   // 跨插件调用
  ],

  // ========== 事件插件必需 ==========
  "subscribedEvents": [
    "plugin:other-plugin:event_name",
    "system:app_ready"
  ],

  // ========== 暴露方法 (供其他插件调用) ==========
  "exposedMethods": ["methodName"],

  // ========== 配置 Schema ==========
  "configSchema": {
    "fieldName": {
      "type": "string",        // string | number | boolean | select
      "required": true,
      "secret": false,         // 是否为敏感信息
      "label": "Field Label",
      "description": "Help text",
      "default": "default value"
    }
  }
}
```

### 字段约束

| 字段 | 类型 | 约束 |
|------|------|------|
| `id` | string | 必需，3-50字符，`^[a-z0-9-]+$` |
| `version` | string | 必需，语义化版本 |
| `apiVersion` | string | 必需，当前仅支持 `"1.0"` |
| `pluginType` | enum | 必需，`data` \| `event` \| `hybrid` |
| `dataType` | enum | data/hybrid类型必需，`usage` \| `balance` \| `status` \| `custom` |
| `refreshIntervalMs` | number | 可选，最小 10000 (10秒) |

---

## 插件 API

### Context 对象

每个插件函数都会收到 `context` 对象，提供以下 API：

```typescript
interface PluginContext {
  // ========== 只读属性 ==========
  readonly pluginId: string;           // 当前插件 ID
  readonly config: Record<string, unknown>;  // 用户配置
  readonly timeout: number;            // 执行超时 (默认 30000ms)
  readonly runtimeApiVersion: string;  // 运行时 API 版本

  // ========== 存储 API (持久化) ==========
  readonly storage: {
    get(key: string): Promise<unknown>;
    set(key: string, value: unknown): Promise<void>;
    delete(key: string): Promise<boolean>;
    keys(): Promise<string[]>;
    clear(): Promise<void>;
  };

  // ========== 缓存 API (内存) ==========
  readonly cache: {
    get(key: string): Promise<unknown | null>;
    set(key: string, value: unknown, ttlMs?: number): Promise<void>;
    delete(key: string): Promise<void>;
    has(key: string): Promise<boolean>;
  };

  // ========== 方法 ==========
  hasCapability(capability: string): boolean;  // 检查权限
  log(level: 'debug' | 'info' | 'warn' | 'error', message: string): void;
  emit(event: string, data?: unknown): void;   // 发送事件
  call(pluginId: string, method: string, params?: unknown): Promise<unknown>;
}
```

### Storage API

持久化存储，数据保存在 `~/.config/aibal/storage/{pluginId}.json`。

```javascript
export async function fetchData(config, context) {
  // 读取上次的数据
  const lastData = await context.storage.get('lastData');

  // 存储新数据
  await context.storage.set('lastData', {
    timestamp: Date.now(),
    value: 100
  });

  // 获取所有键
  const keys = await context.storage.keys();

  // 删除数据
  await context.storage.delete('oldKey');

  // 清空所有数据
  await context.storage.clear();
}
```

**限制**: 每个插件最大 1MB 存储空间。

### Cache API

内存缓存，支持 TTL (默认 5 分钟)。

```javascript
export async function fetchData(config, context) {
  // 检查缓存
  const cached = await context.cache.get('apiResponse');
  if (cached) {
    return cached;
  }

  // 获取新数据
  const data = await fetchFromApi();

  // 缓存 2 分钟
  await context.cache.set('apiResponse', data, 120000);

  return data;
}
```

### Fetch API

沙箱内的安全 fetch 实现。

```javascript
const response = await fetch('https://api.example.com/data', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${config.apiKey}`
  },
  body: JSON.stringify({ key: 'value' })
});

if (!response.ok) {
  throw new Error(`HTTP ${response.status}`);
}

// 解析响应
const text = response.text();    // 获取文本
const json = response.json();    // 解析 JSON
```

**安全限制**:
- 禁止访问私有 IP (127.0.0.1, 192.168.*, 10.*, 等)
- DNS 解析超时 5 秒
- 响应体最大 10MB
- 每个插件最多 10 个并发请求

### 日志 API

```javascript
context.log('debug', '调试信息');
context.log('info', '一般信息');
context.log('warn', '警告信息');
context.log('error', '错误信息');
```

---

## 数据类型

### UsageData (使用量)

```typescript
interface UsageData {
  dataType: 'usage';
  percentage: number;        // 使用百分比 0-100
  used: number;              // 已使用量
  limit: number;             // 总限额
  unit: string;              // 单位: "tokens", "messages", "requests"
  resetTime?: string;        // ISO 8601 重置时间
  resetLabel?: string;       // 重置时间描述: "2小时后重置"
  dimensions?: Array<{       // 多维度数据 (可选)
    id: string;
    label: string;
    percentage: number;
    used: number;
    limit: number;
  }>;
  lastUpdated: string;       // ISO 8601 更新时间
}
```

**示例**:
```javascript
return {
  dataType: 'usage',
  percentage: 75,
  used: 7500,
  limit: 10000,
  unit: 'tokens',
  resetTime: '2025-01-01T00:00:00Z',
  resetLabel: '6小时后重置',
  lastUpdated: new Date().toISOString()
};
```

### BalanceData (余额/配额)

```typescript
interface BalanceData {
  dataType: 'balance';
  balance: number;           // 当前余额
  currency: string;          // 货币: "USD", "CNY"
  quota?: number;            // 总配额
  usedQuota?: number;        // 已用配额
  expiresAt?: string;        // 过期时间
  lastUpdated: string;
}
```

**示例**:
```javascript
return {
  dataType: 'balance',
  balance: 50.00,
  currency: 'USD',
  quota: 100,
  usedQuota: 50,
  lastUpdated: new Date().toISOString()
};
```

### StatusData (状态)

```typescript
interface StatusData {
  dataType: 'status';
  indicator: 'none' | 'minor' | 'major' | 'critical' | 'unknown';
  description: string;
  lastUpdated: string;
}
```

**指示器含义**:
| 值 | 含义 | 建议颜色 |
|----|------|---------|
| `none` | 正常 | 绿色 |
| `minor` | 轻微问题 | 黄色 |
| `major` | 严重问题 | 橙色 |
| `critical` | 严重故障 | 红色 |
| `unknown` | 未知状态 | 灰色 |

---

## 事件系统

### 事件命名规范

```
plugin:{pluginId}:{action}     // 插件事件
system:{action}                 // 系统事件
```

### 发送事件

```javascript
export async function fetchData(config, context) {
  const data = await fetchFromApi();

  // 当使用量超过阈值时发送事件
  if (data.percentage > 80) {
    context.emit('threshold_exceeded', {
      percentage: data.percentage,
      threshold: 80
    });
    // 实际事件名: plugin:my-plugin:threshold_exceeded
  }

  return data;
}
```

### 订阅事件

在 manifest.json 中声明订阅：

```json
{
  "subscribedEvents": [
    "plugin:usage-monitor:threshold_exceeded",
    "system:app_ready",
    "system:refresh_requested"
  ]
}
```

在代码中处理事件：

```javascript
export async function onEvent(event, data, context) {
  switch (event) {
    case 'plugin:usage-monitor:threshold_exceeded':
      await handleThresholdExceeded(data, context);
      break;
    case 'system:app_ready':
      await handleAppReady(context);
      break;
  }
}
```

### 系统事件列表

| 事件 | 说明 | 数据 |
|------|------|------|
| `system:app_ready` | 应用启动完成 | `{}` |
| `system:refresh_requested` | 用户请求刷新 | `{ force: boolean }` |
| `system:config_changed` | 配置变更 | `{ pluginId, config }` |

---

## 跨插件通信

### 暴露方法

在 manifest.json 中声明：

```json
{
  "exposedMethods": ["send", "queue", "clear"]
}
```

在代码中实现：

```javascript
export const exposedMethods = ['send', 'queue', 'clear'];

// 方法实现 (会被沙箱自动调用)
export async function send(params, context) {
  console.log('收到通知请求:', params);
  return { success: true, id: 'notification-123' };
}
```

### 调用其他插件

首先在 manifest.json 中声明权限：

```json
{
  "permissions": [
    "call:notifications:send",
    "call:notifications:queue"
  ]
}
```

然后调用：

```javascript
export async function fetchData(config, context) {
  const data = await fetchFromApi();

  if (data.percentage > 90) {
    // 调用 notifications 插件的 send 方法
    const result = await context.call('notifications', 'send', {
      title: '使用量警告',
      message: `当前使用量: ${data.percentage}%`,
      priority: 'high'
    });

    context.log('info', `通知发送结果: ${result.success}`);
  }

  return data;
}
```

### 调用限制

- **最大调用深度**: 3 层 (A -> B -> C -> 禁止继续)
- **循环检测**: 禁止 A -> B -> A 的循环调用
- **权限检查**: 每次调用都会验证权限

---

## 配置 Schema

### 字段类型

#### String (字符串)

```json
{
  "apiKey": {
    "type": "string",
    "required": true,
    "secret": true,
    "label": "API Key",
    "description": "Your API key from the dashboard"
  }
}
```

#### Number (数字)

```json
{
  "threshold": {
    "type": "number",
    "required": false,
    "default": 80,
    "min": 0,
    "max": 100,
    "label": "警告阈值",
    "description": "当使用量超过此百分比时发出警告"
  }
}
```

#### Boolean (布尔)

```json
{
  "enableNotifications": {
    "type": "boolean",
    "required": false,
    "default": true,
    "label": "启用通知",
    "description": "是否在达到阈值时发送通知"
  }
}
```

#### Select (下拉选择)

```json
{
  "refreshInterval": {
    "type": "select",
    "required": true,
    "default": "60000",
    "label": "刷新间隔",
    "options": [
      { "value": "30000", "label": "30 秒" },
      { "value": "60000", "label": "1 分钟" },
      { "value": "300000", "label": "5 分钟" }
    ]
  }
}
```

### 配置验证

实现 `validateConfig` 函数进行自定义验证：

```javascript
export async function validateConfig(config) {
  // 验证 API Key 格式
  if (!config.apiKey || !config.apiKey.startsWith('sk-')) {
    return {
      valid: false,
      message: 'API Key 格式不正确，应以 sk- 开头'
    };
  }

  // 验证阈值范围
  if (config.threshold && (config.threshold < 0 || config.threshold > 100)) {
    return {
      valid: false,
      message: '阈值必须在 0-100 之间'
    };
  }

  return { valid: true };
}
```

---

## 安全与权限

### 权限类型

| 权限 | 说明 | 声明方式 |
|------|------|---------|
| `network` | 网络请求 (fetch) | `"permissions": ["network"]` |
| `storage` | 持久化存储 | `"permissions": ["storage"]` |
| `cache` | 内存缓存 | `"permissions": ["cache"]` |
| `timer` | setTimeout/setInterval | `"permissions": ["timer"]` |
| `call:{pluginId}:{method}` | 跨插件调用 | `"permissions": ["call:notifications:send"]` |

### 沙箱限制

| 限制项 | 值 |
|--------|-----|
| 内存限制 | 16 MB |
| 栈大小 | 512 KB |
| 执行超时 | 30 秒 |
| 最大并发请求 | 10 |
| 响应体大小 | 10 MB |
| 存储空间 | 1 MB / 插件 |

### 被禁用的 API

以下 JavaScript API 在沙箱中不可用：

- `eval()`
- `Function` 构造函数
- `WebAssembly`
- 原型链修改
- 全局对象修改

### 网络安全

Fetch API 有以下安全限制：

- ❌ 禁止访问 localhost (127.0.0.1, ::1)
- ❌ 禁止访问私有网段 (10.*, 172.16-31.*, 192.168.*)
- ❌ 禁止访问链路本地地址 (169.254.*)
- ✅ DNS 重绑定保护
- ✅ 强制 HTTPS (HTTP 自动升级)

---

## 调试与测试

### 日志调试

使用 `context.log()` 输出日志：

```javascript
export async function fetchData(config, context) {
  context.log('debug', '开始获取数据...');

  try {
    const response = await fetch(config.apiUrl);
    context.log('info', `API 响应状态: ${response.status}`);

    const data = await response.json();
    context.log('debug', `获取到数据: ${JSON.stringify(data)}`);

    return formatData(data);
  } catch (error) {
    context.log('error', `获取数据失败: ${error.message}`);
    throw error;
  }
}
```

日志可在应用的"日志"页面查看。

### 本地开发

1. 将插件放在 `~/.config/aibal/plugins/` 目录
2. 在应用中启用插件
3. 修改代码后，使用"重新加载"功能

---

## 发布指南

### 1. 准备发布

确保你的插件包含：

- [ ] `manifest.json` - 所有必需字段已填写
- [ ] `plugin.js` - 入口文件
- [ ] `icon.png` - 插件图标 (推荐 64x64 PNG)
- [ ] 版本号遵循语义化版本规范

### 2. 生成文件哈希

```bash
# 计算文件的 SHA256 哈希
shasum -a 256 plugin.js
shasum -a 256 icon.png
```

### 3. 打包

```bash
cd my-plugin
zip -r ../my-plugin-1.0.0.zip .
```

### 4. 提交到 Marketplace

联系 AiBal 团队或通过 GitHub 提交 PR 到官方插件仓库。

---

## 完整示例

### Claude API 使用量监控插件

**manifest.json**:
```json
{
  "id": "claude-usage",
  "name": "Claude Usage Monitor",
  "version": "1.0.0",
  "apiVersion": "1.0",
  "pluginType": "data",
  "dataType": "usage",
  "author": "AiBal Community",
  "description": "监控 Claude API 使用量",
  "refreshIntervalMs": 60000,
  "permissions": ["network", "storage"],
  "configSchema": {
    "apiKey": {
      "type": "string",
      "required": true,
      "secret": true,
      "label": "Claude API Key",
      "description": "从 Anthropic Console 获取的 API Key"
    },
    "threshold": {
      "type": "number",
      "required": false,
      "default": 80,
      "min": 0,
      "max": 100,
      "label": "警告阈值 (%)"
    }
  }
}
```

**plugin.js**:
```javascript
export const metadata = {
  id: 'claude-usage',
  name: 'Claude Usage Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'
};

export async function onLoad(context) {
  context.log('info', `${metadata.name} v${metadata.version} 已加载`);
}

export async function fetchData(config, context) {
  context.log('debug', '开始获取 Claude API 使用量...');

  try {
    const response = await fetch('https://api.anthropic.com/v1/usage', {
      headers: {
        'x-api-key': config.apiKey,
        'anthropic-version': '2024-01-01'
      }
    });

    if (!response.ok) {
      throw new Error(`API 错误: ${response.status}`);
    }

    const data = response.json();
    const percentage = Math.round((data.tokens_used / data.tokens_limit) * 100);

    return {
      dataType: 'usage',
      percentage: percentage,
      used: data.tokens_used,
      limit: data.tokens_limit,
      unit: 'tokens',
      lastUpdated: new Date().toISOString()
    };

  } catch (error) {
    context.log('error', `获取数据失败: ${error.message}`);
    throw error;
  }
}

export async function validateConfig(config) {
  if (!config.apiKey) {
    return { valid: false, message: 'API Key 不能为空' };
  }

  if (!config.apiKey.startsWith('sk-ant-')) {
    return {
      valid: false,
      message: 'API Key 格式不正确，应以 sk-ant- 开头'
    };
  }

  return { valid: true };
}
```

---

## 常见问题

### Q: 插件无法加载

**可能原因**:
1. `manifest.json` 格式错误
2. `entry` 指定的文件不存在
3. 必需字段缺失

**解决方法**: 检查应用日志，查看具体错误信息。

### Q: Fetch 请求失败

**可能原因**:
1. 未声明 `network` 权限
2. 目标 URL 被安全策略阻止
3. 网络超时

**解决方法**:
```json
{
  "permissions": ["network"]
}
```

### Q: 跨插件调用失败

**可能原因**:
1. 未声明调用权限
2. 目标插件未暴露该方法
3. 目标插件未启用

**解决方法**:
```json
{
  "permissions": ["call:target-plugin:method-name"]
}
```

### Q: 存储空间不足

**解决方法**:
- 定期清理不需要的数据
- 使用 `context.cache` 存储临时数据
- 压缩存储的数据

### Q: 执行超时

**可能原因**:
1. 网络请求过慢
2. 数据处理过于复杂

**解决方法**:
- 使用缓存减少请求
- 优化数据处理逻辑
- 分批处理大量数据

---

*本文档最后更新: 2025-01-01*
