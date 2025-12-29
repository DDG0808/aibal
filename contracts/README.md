# CUK 契约定义

> **版本**: 1.1.0
> **冻结时间**: 2025-12-27
> **更新时间**: 2025-12-27
> **状态**: FROZEN
> **审核**: Main AI + Codex (xhigh) 评分 92/100 → **Pass** ✅

本目录包含 CUK 插件系统的所有契约定义，是后续所有开发工作的基础。

## 目录结构

```
contracts/
├── README.md                 # 本文件
├── VERSION                   # 契约版本号 (1.1.0)
├── manifest.schema.json      # manifest.json JSON Schema
├── api-version-policy.md     # API 版本兼容策略
├── json-canonical.md         # JSON 规范化规则 (RFC 8785)
├── storage-cache-api.md      # Storage/Cache API 语义规范
├── event-naming.md           # 事件命名规范
├── signature-trust-model.md  # 签名信任模型 (v1.1 新增)
└── types/
    ├── index.d.ts            # 统一导出
    ├── plugin-context.d.ts   # PluginContext API 类型定义
    ├── plugin-module.d.ts    # 插件模块导出契约 (v1.1 新增)
    ├── errors.d.ts           # 错误类型枚举
    ├── ipc-commands.d.ts     # IPC Commands 类型定义 (18个)
    └── ipc-events.d.ts       # IPC Events 类型定义 (6个)
```

## 契约内容

### 1. manifest.json Schema (0.1)

- **文件**: `manifest.schema.json`
- **说明**: 定义插件清单文件的完整结构
- **验证**: 使用 JSON Schema Draft-07

### 2. API 版本策略 (0.1.2)

- **文件**: `api-version-policy.md`
- **说明**: 定义 Major/Minor 版本升级规则和兼容性策略

### 3. JSON 规范化 (0.1.3)

- **文件**: `json-canonical.md`
- **说明**: 定义用于签名的 JSON 规范化规则

### 4. PluginContext API (0.2)

- **文件**: `types/plugin-context.d.ts`, `storage-cache-api.md`
- **说明**: 定义插件运行时上下文 API

### 5. 错误类型 (0.2.3)

- **文件**: `types/errors.d.ts`
- **说明**: 定义 PluginErrorType 枚举

### 6. IPC Commands (0.3.1)

- **文件**: `types/ipc-commands.d.ts`
- **数量**: 18 个 Commands
- **分类**:
  - 插件管理 (9个): plugin_list, plugin_enable, plugin_disable, plugin_install, plugin_uninstall, plugin_reload, plugin_check_updates, plugin_update, plugin_rollback
  - 数据操作 (4个): get_all_data, get_plugin_data, refresh_plugin, refresh_all
  - 配置管理 (3个): get_plugin_config, set_plugin_config, validate_plugin_config
  - 监控 (2个): get_all_health, get_plugin_health

### 7. IPC Events (0.3.2)

- **文件**: `types/ipc-events.d.ts`
- **数量**: 6 个 Events
- **事件**: ipc:plugin_installed, ipc:plugin_uninstalled, ipc:plugin_updated, ipc:plugin_data_updated, ipc:plugin_error, ipc:plugin_health_changed

### 8. 事件命名规范 (0.4)

- **文件**: `event-naming.md`
- **格式**:
  - 插件事件: `plugin:{plugin_id}:{action}` (三段式)
  - 系统事件: `system:{action}` (两段式)
  - IPC 事件: `ipc:{action}` (两段式)

## 使用方式

### TypeScript 项目

```typescript
// 安装类型定义
// npm install @cuk/contracts

// 导入类型
import type {
  PluginContext,
  PluginData,
  IPCCommands,
  PluginErrorType,
} from '@cuk/contracts';
```

### Rust 项目

从 TypeScript 类型定义生成 Rust 类型，或手动保持同步。

## 变更流程

任何契约变更必须遵循 `api-version-policy.md` 定义的流程：

1. 创建 RFC 描述变更
2. 至少 2 人审核
3. 确定版本号变更类型
4. 更新所有相关文档
5. 更新 TypeScript 类型
6. Git Tag 标记新版本
7. 发布变更公告

## 验收 Gate

- [x] 所有契约文档已创建
- [x] JSON Schema 定义完整
- [x] TypeScript 类型定义完整
- [ ] Git Tag 标记 (待后续执行)
- [ ] 变更流程文档化

---

**创建时间**: 2025-12-27
**Phase**: Phase 0 - 契约冻结
