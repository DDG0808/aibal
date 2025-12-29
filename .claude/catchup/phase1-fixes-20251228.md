# Phase 1 问题修复 - 会话总结

> **会话时间**: 2025-12-28 01:20 - 01:30 (UTC+0800)
> **任务**: 修复 Phase 1 审核中发现的 P0/P1 问题
> **状态**: ✅ 全部完成

---

## 一、问题清单

| 优先级 | 问题 | 状态 |
|--------|------|------|
| P0-1 | TypeScript 运行时导出问题 | ✅ 已修复 |
| P0-2 | Rust serde 序列化冲突 | ✅ 已修复 |
| P1-3 | IPC Commands 不匹配 | ✅ 已修复 |
| P1-4 | 依赖膨胀 | ✅ 已修复 |

---

## 二、修复详情

### 2.1 P0-1: TypeScript 运行时导出问题

**问题**: `src/types/index.ts:65` 从 `.d.ts` 文件导出运行时值（`PluginErrorType`、`RETRYABLE_ERRORS`、`isRetryable` 等），但 `.d.ts` 文件在运行时不存在。

**修复**: 将运行时常量和函数在本地定义：

```typescript
// src/types/index.ts

// 只导出类型（使用 export type）
export type { ... } from '@contracts/types';

// 运行时值在本地定义
export enum PluginErrorType { ... }
export const RETRYABLE_ERRORS = [ ... ];
export function isRetryable(errorType: PluginErrorType): boolean { ... }
export function errorTypeFromHttpStatus(status: number): PluginErrorType { ... }
export const CONTRACT_VERSION = '1.1.0';
export const API_VERSION = '1.0';
export const FROZEN_DATE = '2025-12-27';
```

### 2.2 P0-2: Rust serde 序列化冲突

**问题**: `src-tauri/src/plugin/types.rs:318` 中 `PluginData` enum 使用 `#[serde(tag = "dataType")]`，同时各变体结构体又声明了 `data_type: String` 字段，导致 JSON 中出现两个 `dataType`。

**修复**: 移除 `UsageData`、`BalanceData`、`StatusData`、`CustomData` 中的 `data_type` 字段：

```rust
// 修复前
pub struct UsageData {
    pub data_type: String, // "usage" - 与 enum tag 冲突
    ...
}

// 修复后
pub struct UsageData {
    // 注意: data_type 由 PluginData enum 的 #[serde(tag = "dataType")] 自动提供
    ...
}
```

### 2.3 P1-3: IPC Commands 不匹配

**问题**: contracts 定义 18 个 commands，实际仅注册 5 个（`get_version`、`health_check`、`keychain_*`），且这些命令不在契约内。

**修复**: 创建内部 API 契约文件 `contracts/types/internal-commands.d.ts`：

```typescript
// 系统命令 (2个)
export interface SystemCommands {
  get_version(): Promise<string>;
  health_check(): Promise<boolean>;
}

// Keychain 命令 (3个)
export interface KeychainCommands {
  keychain_set(args: {...}): Promise<void>;
  keychain_get(args: {...}): Promise<string | null>;
  keychain_delete(args: {...}): Promise<void>;
}

// 所有内部命令 (5个)
export interface InternalCommands extends SystemCommands, KeychainCommands {}
```

### 2.4 P1-4: 依赖膨胀

**问题**: `Cargo.toml` 引入了 `tokio`、`reqwest`、`notify`、`chrono`、`tracing` 等依赖，但全仓无引用。

**修复**: 移除未使用的依赖，保留注释说明后续 Phase 何时需要：

```toml
# 保留
tauri, tauri-plugin-*, serde, serde_json, log, thiserror, anyhow, security-framework

# 移除（注释保留说明）
# tokio, futures      → Phase 2: QuickJS 集成需要
# reqwest             → Phase 3: 插件商店需要
# notify              → Phase 4: 热更新需要
# chrono              → 可选: 时间处理
# tracing, tracing-subscriber → 已完全移除
```

---

## 三、验收结果

| 检查项 | 结果 |
|--------|------|
| `pnpm run typecheck` | ✅ 通过 |
| `cargo check` | ✅ 通过 (27 warnings - 未使用代码，预期行为) |
| `pnpm test:run` | ✅ 2/2 passed |

---

## 四、变更文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/types/index.ts` | 修改 | 本地定义运行时常量和函数 |
| `src-tauri/src/plugin/types.rs` | 修改 | 移除 4 个结构体的 `data_type` 字段 |
| `contracts/types/internal-commands.d.ts` | 新建 | 内部 API 契约定义 |
| `contracts/types/index.d.ts` | 修改 | 导出内部命令类型 |
| `src-tauri/Cargo.toml` | 修改 | 移除未使用依赖 |

---

## 五、下一步

Phase 1 问题已全部修复，可开始 Phase 2: 插件运行时核心：
- 2.1 QuickJS 集成 (rquickjs 0.6)
- 2.2 沙盒安全层
- 2.3 生命周期管理

---

**会话结束时间**: 2025-12-28 01:30 (UTC+0800)
