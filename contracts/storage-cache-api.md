# Storage & Cache API 语义规范

> 版本: 1.1.0
> 冻结时间: 2025-12-27
> 更新时间: 2025-12-27
> 状态: FROZEN

## 1. 概述

CUK 插件系统提供两种数据存储机制：

| API | 用途 | 持久化 | 位置 | 性能特征 | 一致性保证 |
|-----|------|--------|------|---------|-----------|
| `storage` | 持久化数据 | 是 | 磁盘 | 写入较慢，读取中等 | 最终一致 |
| `cache` | 临时缓存 | 否 | 内存 | 读写快速 | 强一致 |

## 2. Storage API

### 2.1 键空间

- **隔离性**: 每个插件完全隔离，无法访问其他插件的数据
- **内部键名**: `{pluginId}:{key}`
- **用户可见键名**: 仅 `key` 部分

```javascript
// 插件 claude-usage 调用
await context.storage.set('org_id', 'org-xxx');
// 实际存储键: claude-usage:org_id
```

### 2.2 存储位置

```
~/.config/cuk/storage/
├── claude-usage.json
├── claude-status.json
├── notifications.json
└── ...
```

每个插件一个独立 JSON 文件，格式：

```json
{
  "org_id": "org-xxx",
  "last_fetch": "2025-12-27T10:00:00Z",
  "_meta": {
    "version": 1,
    "updated_at": "2025-12-27T10:00:00Z"
  }
}
```

### 2.3 持久化语义

#### 2.3.1 `set()` 完成语义

`storage.set()` 返回的 Promise resolve 时，表示：
- 数据已写入操作系统的文件系统缓冲区
- **不保证**已物理落盘 (fsync)
- 正常应用退出时，数据会被正确持久化

```typescript
await context.storage.set('key', value);
// Promise resolve = 数据已提交到 OS 缓冲区
// 正常退出保证持久化，崩溃可能丢失
```

#### 2.3.2 崩溃一致性

| 场景 | 数据状态 |
|------|----------|
| 正常退出 | ✅ 所有 `set()` 调用的数据都会持久化 |
| 应用崩溃 | ⚠️ 最近约 5 秒内的写入可能丢失 |
| 系统崩溃 | ⚠️ 未 fsync 的数据可能丢失 |

#### 2.3.3 原子性保证

- 单个 `set()` 调用是**原子的**：要么完全写入，要么不写入
- 多个 `set()` 调用之间**没有事务保证**
- 如需批量原子写入，使用单个对象包装：

```javascript
// 非原子（两次写入可能只成功一次）
await context.storage.set('a', 1);
await context.storage.set('b', 2);

// 原子（单次写入）
await context.storage.set('data', { a: 1, b: 2 });
```

### 2.5 并发语义

| 场景 | 行为 |
|------|------|
| 单插件内多次调用 | 串行执行，按调用顺序 |
| 多插件并发写入 | 各自独立文件，无竞态 |
| 读取中写入 | 写入完成后读取获得新值 |

### 2.6 限制

| 限制项 | 值 | 说明 |
|--------|-----|------|
| 单插件总大小 | 1 MB | 超出拒绝写入 |
| 单个值大小 | 100 KB | 超出拒绝写入 |
| 键名长度 | 256 字符 | 超出拒绝 |
| 键名字符 | `[a-zA-Z0-9_-]` | 其他字符拒绝 |
| 键数量 | 1000 | 超出需清理旧键 |

### 2.5 迁移策略

当需要更改存储结构时：

```javascript
const STORAGE_VERSION = 2;

export async function onLoad(context) {
  const meta = await context.storage.get('_meta');

  if (!meta || meta.version < STORAGE_VERSION) {
    await migrateStorage(context, meta?.version || 0);
    await context.storage.set('_meta', { version: STORAGE_VERSION });
  }
}

async function migrateStorage(context, fromVersion) {
  if (fromVersion < 2) {
    // v1 -> v2: 重命名字段
    const oldValue = await context.storage.get('old_key');
    if (oldValue !== undefined) {
      await context.storage.set('new_key', oldValue);
      await context.storage.delete('old_key');
    }
  }
}
```

### 2.6 错误处理

```typescript
try {
  await context.storage.set('key', value);
} catch (error) {
  if (error.type === PluginErrorType.STORAGE_LIMIT) {
    // 存储空间不足
    context.log('warn', 'Storage limit reached, clearing old data');
    await context.storage.clear();
  }
}
```

## 3. Cache API

### 3.1 键空间

- **隔离性**: 每个插件完全隔离
- **内部键名**: `cache:{pluginId}:{key}`
- **用户可见键名**: 仅 `key` 部分

### 3.2 过期策略

使用 moka 缓存库，支持两种过期机制：

| 策略 | 说明 | 默认值 |
|------|------|--------|
| TTL (Time-To-Live) | 从写入开始计时 | 300000ms (5分钟) |
| TTI (Time-To-Idle) | 从最后访问开始计时 | 120000ms (2分钟) |

缓存项在以下情况下过期：
1. 超过 TTL 时间（从写入算起）
2. 超过 TTI 时间无访问

### 3.3 并发语义

| 场景 | 行为 |
|------|------|
| 并发读取 | 完全并发，高性能 |
| 并发写入同一键 | 后写入覆盖前写入 |
| 读取时过期 | 返回 null |
| 删除时读取 | 可能返回旧值或 null |

### 3.4 限制

| 限制项 | 值 | 说明 |
|--------|-----|------|
| 全局缓存大小 | 100 MB | 所有插件共享 |
| 单个值大小 | 10 MB | 超出拒绝 |
| 键名长度 | 256 字符 | 超出拒绝 |
| TTL 范围 | 1000-3600000 ms | 1秒-1小时 |

### 3.5 使用模式

#### 请求缓存

```javascript
export async function fetchData(config, context) {
  const cacheKey = 'usage_data';

  // 尝试从缓存获取
  const cached = await context.cache.get(cacheKey);
  if (cached) {
    return cached;
  }

  // 发起请求
  const data = await fetch('https://api.example.com/usage');

  // 写入缓存
  await context.cache.set(cacheKey, data, 60000); // 1分钟

  return data;
}
```

#### 组织 ID 缓存

```javascript
async function getOrgId(config, context) {
  // 先查缓存
  let orgId = await context.cache.get('org_id');
  if (orgId) return orgId;

  // 再查持久存储
  orgId = await context.storage.get('org_id');
  if (orgId) {
    // 写入缓存加速后续访问
    await context.cache.set('org_id', orgId, 3600000); // 1小时
    return orgId;
  }

  // 都没有，请求 API
  orgId = await fetchOrgIdFromApi(config);

  // 同时写入存储和缓存
  await context.storage.set('org_id', orgId);
  await context.cache.set('org_id', orgId, 3600000);

  return orgId;
}
```

## 4. Storage vs Cache 选择指南

| 场景 | 推荐 API | 原因 |
|------|----------|------|
| 用户凭证 | `storage` | 需要持久化，应用重启后保留 |
| API 响应 | `cache` | 临时数据，可重新获取 |
| 组织 ID | `storage` + `cache` | 持久化但频繁访问 |
| 请求去重 | `cache` | 短期去重，无需持久化 |
| 上次刷新时间 | `storage` | 需要跨会话保留 |
| 计算中间结果 | `cache` | 临时数据 |

## 5. 最佳实践

### 5.1 键名命名

```javascript
// 好的命名
'org_id'              // 简洁明了
'last_fetch_time'     // 下划线分隔
'usage-2025-12-27'    // 连字符分隔日期

// 避免的命名
'a'                   // 太短，不够描述性
'myOrganizationIdentifierFromClaudeAPI'  // 太长
'org.id'              // 包含不允许的字符
```

### 5.2 序列化规范

Storage 和 Cache 只能存储 **JSON 可序列化值**。以下是完整的类型定义：

```typescript
/**
 * JSON 可序列化值类型
 * storage.set() 和 cache.set() 的 value 参数必须符合此类型
 */
type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };
```

#### 允许的值类型

| 类型 | 示例 | 说明 |
|------|------|------|
| string | `"hello"` | UTF-8 字符串 |
| number | `42`, `3.14`, `-1` | 有限数字 (非 NaN/Infinity) |
| boolean | `true`, `false` | 布尔值 |
| null | `null` | 空值 |
| array | `[1, 2, 3]` | 数组 (元素也需可序列化) |
| object | `{ "a": 1 }` | 对象 (值也需可序列化) |

#### 不允许的值类型

| 类型 | 示例 | 错误处理 |
|------|------|---------|
| undefined | `undefined` | 对象中会被忽略，顶层报错 |
| function | `() => {}` | 抛出 STORAGE_LIMIT 错误 |
| Symbol | `Symbol('x')` | 抛出 STORAGE_LIMIT 错误 |
| BigInt | `100n` | 抛出 STORAGE_LIMIT 错误 |
| Date | `new Date()` | 抛出 STORAGE_LIMIT 错误 (应转为 ISO 字符串) |
| Map/Set | `new Map()` | 抛出 STORAGE_LIMIT 错误 |
| 循环引用 | `a.b = a` | 抛出 STORAGE_LIMIT 错误 |
| NaN/Infinity | `NaN`, `Infinity` | 抛出 STORAGE_LIMIT 错误 |

#### 代码示例

```javascript
// ✅ 可以存储
await context.storage.set('data', {
  name: 'test',
  count: 42,
  active: true,
  items: ['a', 'b'],
  nested: { key: 'value' },
  timestamp: new Date().toISOString(), // Date 转为字符串
});

// ❌ 不能存储（会失败）
await context.storage.set('data', {
  date: new Date(),      // 应使用 .toISOString()
  func: () => {},        // 函数不可序列化
  sym: Symbol('test'),   // Symbol 不可序列化
  bigint: 100n,          // BigInt 不可序列化 (可用 String(100n))
});
```

### 5.3 错误处理

始终处理存储/缓存操作可能的错误：

```javascript
export async function fetchData(config, context) {
  try {
    const cached = await context.cache.get('data');
    if (cached) return cached;
  } catch (e) {
    context.log('warn', `Cache read failed: ${e.message}`);
    // 继续执行，从 API 获取
  }

  const data = await fetchFromApi(config);

  try {
    await context.cache.set('data', data, 60000);
  } catch (e) {
    context.log('warn', `Cache write failed: ${e.message}`);
    // 不影响返回数据
  }

  return data;
}
```

## 6. 变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2025-12-27 | 初始冻结版本 |
| 1.1.0 | 2025-12-27 | 基于 Codex 审核修订：明确持久化语义、增强序列化规范 |
