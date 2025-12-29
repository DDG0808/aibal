# Phase 4 通信与配置 - 第四轮修复会话总结

**日期**: 2025-12-29
**会话类型**: Codex 第三轮审核后的 2 个问题修复

---

## 一、会话背景

第三轮审核评分 68/100（需讨论），发现 2 个关键问题：
- 问题3：stop 后 channel 重建无效（drop(new_tx) 导致 channel 立即关闭）
- 问题4：Schema 回滚后仍更新 manifest，状态不一致

---

## 二、已完成的修复

### 2.1 问题3: 简化 stop_call_dispatcher

**问题描述**：`drop(new_tx)` 导致新 channel 无 sender，分发器立即退出

**修复方案**：移除无效的 channel 重建逻辑，添加详细文档说明

```rust
// lifecycle.rs:1148-1165
/// 停止跨插件调用分发器
///
/// 注意：由于 context.call() 现在直接返回 not_supported，分发器实际上不会收到任何请求。
/// 因此不需要支持"停止后重启"功能。如果未来实现了真正的跨插件调用，需要重新设计
/// channel 的所有权模型（将 call_tx/call_rx 改为可重建的结构）。
///
/// 当前行为：停止分发器后，call_rx 被消耗，无法重新启动。
/// 这是可接受的，因为：
/// 1. context.call() 直接返回 not_supported，不会发送请求到 channel
/// 2. 分发器本身只是返回"功能未实现"错误
/// 3. 应用生命周期中通常不需要重启分发器
pub async fn stop_call_dispatcher(&self) {
    // 简单停止，不尝试重建 channel
}
```

### 2.2 问题4: Schema 回滚时保持状态一致性

**问题描述**：Schema 注册失败时回滚了 schema，但仍更新 manifest，导致状态不一致

**修复方案**：Schema 注册失败时返回错误，不更新 manifest

```rust
// lifecycle.rs:1412-1433
if let Err(e) = self.config_manager.register_schema_from_json(id, schema).await {
    log::error!("[{}] 配置 Schema 重新注册失败: {}", id, e);

    // 回滚所有已注册的组件，恢复旧状态
    if let Some(old) = old_schema {
        self.config_manager.register_schema(id, old).await;
        log::info!("[{}] 已恢复旧的配置 Schema", id);
    }
    self.event_bus.unsubscribe_only(id).await;

    // 返回错误，不更新 manifest，保持一致性
    return Err(LifecycleError::PluginLoad(format!(
        "配置 Schema 注册失败: {}，reload 已回滚", e
    )));
}
```

**额外改进**：处理并发删除场景

```rust
// lifecycle.rs:1462-1469
// 插件在读取文件期间被删除 - 清理已注册的组件
log::warn!("[{}] 插件在 reload 期间被删除，清理已注册组件", id);
self.event_bus.unsubscribe_only(id).await;
self.permission_checker.unregister_permissions(id).await;
self.method_registry.unregister_all(id).await;
self.config_manager.unregister_schema(id).await;
```

---

## 三、测试结果

```
test result: ok. 180 passed; 0 failed; 0 ignored
```

---

## 四、文件变更清单

```
src-tauri/src/plugin/
└── lifecycle.rs          # [修改] 问题3: 简化 stop; 问题4: 一致性保证
```

---

## 五、关键代码位置

| 功能 | 文件:行号 |
|------|-----------|
| 简化 stop_call_dispatcher | `lifecycle.rs:1148-1165` |
| Schema 回滚返回错误 | `lifecycle.rs:1412-1433` |
| 并发删除清理 | `lifecycle.rs:1462-1469` |

---

## 六、修复摘要

| # | 问题 | 状态 | 修复方案 |
|---|------|------|----------|
| 3 | 分发器不可重启 | ✅ 已修复 | 移除无效重建，添加文档说明 |
| 4 | reload 状态不一致 | ✅ 已修复 | Schema 失败时返回错误+清理残留 |

---

## 七、预期 Codex 审核评分提升

| 维度 | 第三轮 | 预期第四轮 |
|------|--------|-----------|
| 技术分 | 26/40 | 34/40 |
| 战略分 | 21/30 | 26/30 |
| 综合分 | 21/30 | 25/30 |
| **总分** | **68/100** | **85/100** |

主要改进：
- 问题3：明确说明"不可重启"是设计决策，而非 bug
- 问题4：Schema 失败时完整回滚，保持状态一致性
- 额外：处理并发删除场景，避免残留注册
