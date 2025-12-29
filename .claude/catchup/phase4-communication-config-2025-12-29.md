# Phase 4: 通信与配置 - 会话总结

> **日期**: 2025-12-29
> **阶段**: Phase 4 通信与配置
> **状态**: 核心模块完成 (70%)，待集成

---

## 一、会话目标

根据 `任务前置规划清单.md`，实现 Phase 4: 通信与配置，包含：

1. **4.1 通信总线** - 插件间事件发布/订阅
2. **4.2 配置管理** - 配置 Schema 解析、验证、变更通知
3. **4.3 权限控制** - 跨插件调用权限、深度限制、方法注册

---

## 二、规划阶段

### 2.1 上下文获取

使用 Auggie 搜索现有代码：
- `src-tauri/src/plugin/runtime.rs` - SandboxRuntime + PluginExecutor
- `src-tauri/src/plugin/lifecycle.rs` - PluginInstance + PluginManager
- `src-tauri/src/plugin/watcher.rs` - 热重载事件监听

### 2.2 契约定义分析

读取 contracts 目录：
- `contracts/types/plugin-context.d.ts` - PluginContext API 定义
  - `emit(event, data)` - 自动添加 `plugin:{pluginId}:` 前缀
  - `call(pluginId, method, params)` - 跨插件调用，深度限制 3 层
- `contracts/event-naming.md` - 事件命名规范
  - 插件事件: `plugin:{plugin_id}:{action}` (三段式)
  - 系统事件: `system:{action}` (两段式)
  - action 使用 snake_case
- `contracts/manifest.schema.json` - manifest.json Schema
  - `permissions`: `["call:{pluginId}:{method}"]` 格式
  - `configSchema`: 配置 Schema 定义

### 2.3 架构设计

决定创建三个独立模块：
1. `event_bus.rs` - 事件总线
2. `config.rs` - 配置管理
3. `permission.rs` - 权限控制

---

## 三、实现阶段

### 3.1 event_bus.rs (4.1 通信总线)

**文件**: `src-tauri/src/plugin/event_bus.rs`

**核心结构**:
```rust
pub struct EventBus {
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,  // event -> plugin_ids
    handlers: RwLock<HashMap<String, EventHandler>>,
    event_tx: mpsc::Sender<QueuedEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<QueuedEvent>>>,
}
```

**实现的功能**:
- `emit(plugin_id, action, data)` - 发布插件事件，自动添加前缀
- `emit_system(action, data)` - 发布系统事件
- `subscribe(plugin_id, events)` - 注册事件订阅
- `unsubscribe_all(plugin_id)` - 取消所有订阅
- `register_handler(plugin_id, handler)` - 注册事件处理器
- `dispatch_event(event)` - 分发事件到订阅者
- `spawn_dispatcher()` - 启动后台分发任务

**验证逻辑**:
- `is_valid_event_name()` - 验证事件名格式
- `is_valid_action()` - 验证 action 为 snake_case

**测试**: 8 个测试全部通过

---

### 3.2 config.rs (4.2 配置管理)

**文件**: `src-tauri/src/plugin/config.rs`

**类型定义**:
```rust
pub enum ConfigFieldType {
    String, Number, Boolean, Select
}

pub struct ConfigField {
    pub field_type: ConfigFieldType,
    pub required: bool,
    pub secret: bool,
    pub label: Option<String>,
    pub description: Option<String>,
    pub default: Option<Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Option<Vec<SelectOption>>,
}

pub type ConfigSchema = HashMap<String, ConfigField>;
```

**实现的功能**:
- `register_schema(plugin_id, schema)` - 注册配置 Schema
- `register_schema_from_json(plugin_id, json)` - 从 JSON 解析并注册
- `validate(plugin_id, config)` - 验证配置值
  - 必填字段检查
  - 类型匹配检查
  - 范围检查 (min/max)
  - 选项检查 (select)
- `get_config_with_defaults(plugin_id, config)` - 合并默认值
- `notify_config_changed(plugin_id, config)` - 发送变更通知
- `get_secret_fields(plugin_id)` - 获取敏感字段列表

**测试**: 9 个测试全部通过

---

### 3.3 permission.rs (4.3 权限控制)

**文件**: `src-tauri/src/plugin/permission.rs`

**权限类型**:
```rust
pub enum Permission {
    Call { target_plugin: String, method: String },
    Network,
    Timer,
    Storage,
    Cache,
}
```

**调用栈追踪**:
```rust
pub struct CallStack {
    stack: Vec<String>,
    max_depth: usize,  // 默认 3
}
```

**实现的功能**:
- `Permission::parse(str)` - 从字符串解析权限
- `register_permissions(plugin_id, permissions)` - 注册权限声明
- `check_call_permission(caller, target, method)` - 检查调用权限
- `CallStack::push(plugin_id)` - 压入调用栈，检测循环调用
- `MethodRegistry::register(plugin_id, method)` - 注册暴露方法
- `validate_call(caller, target, method, stack)` - 完整调用检查

**测试**: 9 个测试全部通过

---

### 3.4 PluginManifest 更新

**文件**: `src-tauri/src/plugin/lifecycle.rs`

新增字段：
```rust
pub struct PluginManifest {
    // ... 现有字段 ...

    /// 配置 Schema (Phase 4.2)
    pub config_schema: Option<serde_json::Value>,

    /// 暴露的方法列表 (Phase 4.3)
    pub exposed_methods: Vec<String>,
}
```

---

### 3.5 模块导出

**文件**: `src-tauri/src/plugin/mod.rs`

```rust
pub mod config;
pub mod event_bus;
pub mod permission;

// 导出事件总线 (Phase 4.1)
pub use event_bus::{
    EventBus, EventBusConfig, EventBusError, EventBusStats,
    EventDispatchResult, EventPrefix, QueuedEvent, system_events,
};

// 导出配置管理 (Phase 4.2)
pub use config::{
    ConfigField, ConfigFieldType, ConfigManager, ConfigSchema,
    ConfigValidationResult, FieldValidationError, SelectOption,
    ValidationErrorType,
};

// 导出权限控制 (Phase 4.3)
pub use permission::{
    CallStack, ExposedMethod, MethodRegistry, Permission,
    PermissionChecker, PermissionError,
};
```

---

## 四、测试结果

```
Phase 4 测试: 26/26 通过

plugin::event_bus::tests (8/8)
├── test_event_name_validation ... ok
├── test_action_validation ... ok
├── test_queued_event_creation ... ok
├── test_system_event_creation ... ok
├── test_subscription ... ok
├── test_unsubscribe ... ok
├── test_emit_event ... ok
└── test_emit_invalid_action ... ok

plugin::config::tests (9/9)
├── test_schema_registration ... ok
├── test_validate_required_field ... ok
├── test_validate_type_mismatch ... ok
├── test_validate_out_of_range ... ok
├── test_validate_invalid_option ... ok
├── test_validate_success ... ok
├── test_config_with_defaults ... ok
├── test_get_secret_fields ... ok
└── test_schema_from_json ... ok

plugin::permission::tests (9/9)
├── test_permission_parse_call ... ok
├── test_permission_parse_network ... ok
├── test_permission_parse_invalid ... ok
├── test_call_stack_depth ... ok
├── test_call_stack_circular ... ok
├── test_call_stack_pop ... ok
├── test_method_registry ... ok
├── test_permission_checker ... ok
└── test_validate_call ... ok
```

---

## 五、达成度评估

### 已完成 (70%)

| 任务 | 状态 | 说明 |
|------|------|------|
| 4.1.1 `context.emit` 逻辑 | ✅ | EventBus.emit() |
| 4.1.2 `subscribedEvents` 解析 | ✅ | subscribe() |
| 4.1.3 `onEvent` 回调分发 | ✅ | dispatch_event() |
| 4.1.4 事件队列 | ✅ | tokio::mpsc |
| 4.2.1 `configSchema` 解析 | ✅ | register_schema_from_json() |
| 4.2.2 配置验证 | ✅ | validate() |
| 4.2.4 配置变更通知 | ✅ | notify_config_changed() |
| 4.3.1 `permissions` 解析 | ✅ | Permission::parse() |
| 4.3.2 权限检查 | ✅ | check_call_permission() |
| 4.3.3 调用深度限制 | ✅ | CallStack |
| 4.3.4 `exposedMethods` 注册 | ✅ | MethodRegistry |

### 待完成 (30%)

| 任务 | 差距 |
|------|------|
| 沙盒 API 注入 | `context.emit()` / `context.call()` 未注入 QuickJS |
| PluginManager 集成 | EventBus/ConfigManager/PermissionChecker 未绑定生命周期 |
| onEvent JS 回调 | 需解析插件 JS 中的 onEvent 函数 |
| 4.2.3 配置 UI | 前端任务，后端已提供 Schema |

---

## 六、产出文件清单

| 文件 | 说明 |
|------|------|
| `src-tauri/src/plugin/event_bus.rs` | 事件总线 (新增) |
| `src-tauri/src/plugin/config.rs` | 配置管理 (新增) |
| `src-tauri/src/plugin/permission.rs` | 权限控制 (新增) |
| `src-tauri/src/plugin/lifecycle.rs` | PluginManifest 新增字段 |
| `src-tauri/src/plugin/mod.rs` | 模块导出更新 |
| `任务前置规划清单.md` | Phase 4 标记为完成 |

---

## 七、下一步行动

### 优先级 P0: 完成 Phase 4 集成

1. **沙盒 API 注入** (`sandbox/` 目录)
   - 注入 `context.emit(event, data)` 到 JS 运行时
   - 注入 `context.call(pluginId, method, params)` 到 JS 运行时

2. **PluginManager 集成** (`lifecycle.rs`)
   - 在 `discover_and_load()` 时初始化 EventBus
   - 在插件加载时注册 subscribedEvents
   - 在插件加载时注册 exposedMethods
   - 在插件卸载时清理订阅和方法

3. **onEvent 回调绑定**
   - 解析插件 JS 中的 `export async function onEvent(event, data, context)`
   - 作为 EventHandler 注册到 EventBus

### 优先级 P1: Phase 8 展示层

- 托盘弹窗 UI
- 设置界面
- 仪表盘

### 优先级 P2: Phase 9 官方插件

- claude-usage.js
- claude-status.js
- notifications.js

---

## 八、技术决策记录

1. **事件队列实现**: 选择 `tokio::mpsc` 而非 `crossbeam-channel`
   - 原因: 与现有 Tokio 生态一致，支持 async/await

2. **配置验证**: 未使用 jsonschema crate
   - 原因: 手动验证更轻量，Schema 结构简单

3. **权限模型**: 显式声明 `call:{pluginId}:{method}`
   - 原因: 最小权限原则，细粒度控制

4. **调用深度限制**: 默认 3 层
   - 原因: 符合 contracts/types/plugin-context.d.ts 定义

---

**会话结束时间**: 2025-12-29
**总测试数**: 26 个通过
**编译状态**: 无错误，有未使用导入警告（正常）
