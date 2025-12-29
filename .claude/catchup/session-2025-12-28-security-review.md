# 会话总结: 安全审查修复（P1/P2 问题）

**日期**: 2025-12-28
**会话类型**: 安全修复
**范围**: `src-tauri/src/plugin/sandbox/timer.rs`, `src-tauri/src/plugin/lifecycle.rs`

---

## 1. 修复的问题

| 级别 | 问题 | 修复方案 | 文件 |
|------|------|----------|------|
| P1 | pending_tokens 窗口期漏取消 | register() 返回 bool 检查竞态 | `timer.rs:171-202` |
| P1 | 中间目录 TOCTOU | openat 链式验证 | `lifecycle.rs:38-178` |
| P2 | Mutex unwrap() panic 风险 | recover_lock() 恢复毒化锁 | `timer.rs:26-39` |
| P2 | 缺少回归测试 | 添加竞态/毒化/symlink 测试 | `timer.rs:486-577`, `lifecycle.rs:1187-1271` |

---

## 2. 详细变更

### 2.1 P1: pending_tokens 竞态修复 (`timer.rs`)

**问题**：`cancel()` 从 `pending_tokens` 删除 token 后，`register()` 仍可能将其存入 `timers`，导致取消失效。

**解决方案**：`register()` 返回 `bool`，检查 id 是否仍在 pending 中：
```rust
pub async fn register(...) -> bool {
    let was_in_pending = {
        let mut pending = recover_lock(self.pending_tokens.lock());
        pending.remove(&id).is_some()
    };
    if !was_in_pending {
        return false; // 已被取消
    }
    // ... 继续注册
    true
}
```

调用方检查返回值：
```rust
if !registry_clone.register(...).await {
    log::trace!("定时器 {} 在竞态窗口期被取消", id);
    return;
}
```

### 2.2 P1: TOCTOU openat 修复 (`lifecycle.rs`)

**问题**：传统 symlink_metadata 检查存在 TOCTOU 窗口：
1. 检查 `plugins/foo` 不是 symlink
2. 攻击者替换 `foo` 为 symlink → `/etc`
3. 代码使用 `plugins/foo/entry.js` → `/etc/entry.js`

**解决方案**：使用 `openat` 链式打开（Unix）：
```rust
// 通过 fd 链式打开，攻击者无法在打开后替换
let base_fd = libc::open(base_dir, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
for component in dirs {
    let next_fd = libc::openat(current_fd, component, O_NOFOLLOW | O_DIRECTORY);
    // 如果 component 是 symlink，openat 返回错误
}
```

### 2.3 P2: Mutex 毒化恢复 (`timer.rs`)

**问题**：`lock().unwrap()` 在锁毒化时会 panic，导致连锁崩溃。

**解决方案**：添加 `recover_lock()` 辅助函数：
```rust
fn recover_lock<'a, T>(result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>)
    -> MutexGuard<'a, T>
{
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("Mutex 已毒化，恢复数据继续操作");
            poisoned.into_inner()
        }
    }
}
```

替换所有 5 处 `.lock().unwrap()` 调用。

### 2.4 P2: 回归测试

新增测试：

| 测试 | 说明 |
|------|------|
| `test_cancel_before_register_race` | 验证 register 前取消 |
| `test_cancel_during_register_window` | 100 次并发竞态测试 |
| `test_recover_lock_normal` | 正常锁恢复 |
| `test_recover_lock_poisoned` | 毒化锁恢复 |
| `test_openat_verifier_normal_path` | 正常路径通过 |
| `test_openat_verifier_rejects_symlink` | symlink 目录拒绝 |
| `test_openat_verifier_rejects_symlink_file` | symlink 文件拒绝 |
| `test_entry_path_rejects_absolute_path` | 绝对路径拒绝 |
| `test_entry_path_rejects_path_traversal` | 路径遍历拒绝 |

---

## 3. 验证结果

```
cargo check: ✅ 成功（仅警告）
cargo test:  ✅ 84 passed
```

---

## 4. 产出文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/plugin/sandbox/timer.rs` | +70 行（recover_lock + 竞态修复 + 测试） |
| `src-tauri/src/plugin/lifecycle.rs` | +190 行（openat_verifier + 测试） |

---

## 5. 安全改进总结

| 维度 | 改进 |
|------|------|
| **竞态安全** | pending_tokens 双重检查消除 ABA 问题 |
| **TOCTOU 防护** | openat 链式验证消除中间目录竞态 |
| **稳定性** | Mutex 毒化恢复避免连锁 panic |
| **可测试性** | 完整回归测试覆盖边缘情况 |

---

## 6. 后续建议

1. **Windows TOCTOU**：当前使用 symlink_metadata fallback，攻击面较小（需管理员权限）
2. **性能监控**：openat 多次系统调用可能影响性能，建议添加缓存
3. **Codex 复审**：建议使用 Codex 进行新一轮审查，验证评分提升

---

**会话总结生成时间**: 2025-12-28T21:50:00+08:00
