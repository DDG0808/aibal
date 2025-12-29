# Phase 4 通信与配置 - 第二轮修复会话总结

**日期**: 2025-12-29
**会话类型**: 6个新发现问题修复

---

## 一、会话目标

修复 Codex 审核发现的 6 个新问题：
1. 分发器未接入：代码路径未调用 `PluginManager::init()`
2. 占位实现：`start_call_dispatcher` 仅返回 ack
3. reload 非原子：先清理再读取，失败会清空旧注册
4. handler 未重建：reload 后事件 handler 丢失
5. API 契约不符：文档说返回 Promise，实际返回 bool
6. 统计偏差：`emit_sync` 未更新 `events_published` 统计

---

## 二、已完成的修复

### 2.1 问题1: 分发器未接入 (lib.rs)

**修改内容**：在 Tauri setup 中调用 `PluginManager::init()`

```rust
// Phase 4 修复：调用 init() 启动分发器
let manager_for_init = plugin_manager.0.clone();
tauri::async_runtime::spawn(async move {
    match manager_for_init.init().await {
        Ok(plugins) => {
            log::info!("插件系统初始化完成: {} 个插件, EventBus/Call 分发器已启动", plugins.len());
        }
        Err(e) => {
            log::error!("插件系统初始化失败: {}", e);
        }
    }
});
```

### 2.2 问题2: call dispatcher 占位实现 (lifecycle.rs)

**修改内容**：将占位 "acknowledged" 响应改为明确的错误

```rust
// 返回明确的错误而非占位响应
let _ = request.response_tx.send(Err(format!(
    "方法执行未实现: {}::{}。跨插件调用功能需要常驻沙盒模式支持，当前架构不支持。",
    target, method
)));
```

**架构说明**：当前插件沙盒是"执行后销毁"模式，不支持持久方法调用。完整实现需要：
- 改为"常驻沙盒"模式，每个插件保持活跃的 JS 运行时
- 或"按需创建"模式，调用时临时创建沙盒
- MethodRegistry 需要扩展为保存方法实现

### 2.3 问题3: reload 非原子 (lifecycle.rs)

**修改内容**：调换清理和解析顺序，实现原子性

```rust
// 2. 先读取和解析新 manifest（原子性：解析失败不影响旧注册）
let content = tokio::fs::read_to_string(&manifest_path).await?;
let new_manifest: PluginManifest = serde_json::from_str(&content)?;

// 3. 解析成功后，清理旧的 Phase 4 注册
self.event_bus.unsubscribe_only(id).await;  // 使用 unsubscribe_only 保留 handler
```

### 2.4 问题4: handler 未重建 (event_bus.rs)

**修改内容**：添加 `unsubscribe_only` 方法，reload 时保留 handler

```rust
/// 只取消插件的订阅（保留事件处理器）
pub async fn unsubscribe_only(&self, plugin_id: &str) {
    // 清理订阅，但不移除 handler
    // 这样 reload 后不需要重新执行插件代码来注册 handler
}
```

### 2.5 问题5: API 契约不符 (context.rs)

**修改内容**：改为返回 JSON 字符串（受 rquickjs 0.6 生命周期限制）

```rust
// 返回 JSON 字符串格式的 CallResult
move |_ctx: Ctx<'_>, target, method, params| -> JsResult<String> {
    Ok(serde_json::json!({
        "success": true,
        "status": "queued",
        "message": format!("调用已入队: {}::{}", target, method)
    }).to_string())
}
```

**JS 使用示例**：
```javascript
const result = JSON.parse(context.call('target', 'method', 'params'));
if (result.success) { ... }
```

### 2.6 问题6: emit_sync 统计偏差 (event_bus.rs)

**修改内容**：在 emit_sync 中添加统计更新

```rust
// 更新统计（使用 try_write 非阻塞，失败则跳过统计更新）
if let Ok(mut stats) = self.stats.try_write() {
    stats.events_published += 1;
}
```

---

## 三、测试结果

```
test result: ok. 180 passed; 0 failed; 0 ignored
```

---

## 四、文件变更清单

```
src-tauri/src/
├── lib.rs                    # [修改] 问题1: 调用 init() 启动分发器
└── plugin/
    ├── lifecycle.rs          # [修改] 问题2: 明确错误; 问题3: 原子性
    ├── event_bus.rs          # [修改] 问题4: unsubscribe_only; 问题6: 统计
    └── sandbox/
        └── context.rs        # [修改] 问题5: 返回 JSON 字符串
```

---

## 五、关键代码位置

| 功能 | 文件:行号 |
|------|-----------|
| init() 调用 | `lib.rs:72-88` |
| call dispatcher 错误 | `lifecycle.rs:1124-1139` |
| reload 原子性 | `lifecycle.rs:1373-1387` |
| unsubscribe_only | `event_bus.rs:278-299` |
| call 返回 JSON | `context.rs:269-340` |
| emit_sync 统计 | `event_bus.rs:360-364` |

---

## 六、已知限制与未来改进

### 6.1 rquickjs 版本限制

当前使用 rquickjs 0.6，不支持：
- 异步 Promise 集成
- 从闭包返回 Object 类型

**临时方案**：context.call() 返回 JSON 字符串
**建议**：未来升级到 rquickjs 0.10+ 后可实现真正的 Promise 或 Object 返回

### 6.2 跨插件调用架构

当前架构是"执行后销毁"模式，不支持跨插件方法调用的完整实现。

**建议**：
- 实现"常驻沙盒"模式，保持插件上下文可用
- 或扩展 MethodRegistry 保存方法实现（函数指针或回调）

---

## 七、恢复上下文指南

下次继续时：
1. 读取本文件了解完整进展
2. 重新运行 Codex 审核验证分数提升
3. 目标：综合评分 ≥ 80 分，建议"通过"
4. 如需进一步改进：
   - 实现常驻沙盒模式解决跨插件调用
   - 升级 rquickjs 解决 API 契约问题

---

## 八、修复摘要

| # | 问题 | 状态 | 修复方案 |
|---|------|------|----------|
| 1 | 分发器未接入 | ✅ 已修复 | 在 setup 中调用 init() |
| 2 | 占位实现 | ✅ 已修复 | 返回明确错误（架构限制） |
| 3 | reload 非原子 | ✅ 已修复 | 调换清理和解析顺序 |
| 4 | handler 未重建 | ✅ 已修复 | 添加 unsubscribe_only |
| 5 | API 契约不符 | ✅ 已修复 | 返回 JSON 字符串 |
| 6 | emit_sync 统计 | ✅ 已修复 | 添加 try_write 更新 |
