# 会话总结: 安全审查修复 + Codex 复审

**日期**: 2025-12-28
**会话类型**: 安全修复 + 代码审查
**范围**: `src-tauri/src/plugin/` 目录

---

## 1. 会话目标

根据上次 Codex 审核（68分）发现的问题，修复以下 4 个安全/稳定性问题：

| 级别 | 问题 | 状态 |
|------|------|------|
| P1 | pending_tokens 窗口期漏取消 | ✅ 已修复 |
| P1 | 中间目录 TOCTOU（需 openat 方案） | ⚠️ 部分修复 |
| P2 | std::sync::Mutex unwrap() panic 风险 | ✅ 已修复 |
| P2 | 缺少回归测试 | ✅ 已添加 |

---

## 2. 完成的工作

### 2.1 P2: Mutex 毒化恢复 (`timer.rs:26-39`)

**问题**: `.lock().unwrap()` 在锁毒化时会连锁 panic

**修复**: 添加 `recover_lock()` 辅助函数
```rust
fn recover_lock<'a, T>(
    result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>
) -> MutexGuard<'a, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("Mutex 已毒化，恢复数据继续操作");
            poisoned.into_inner()
        }
    }
}
```

替换了 5 处 `.lock().unwrap()` 调用。

### 2.2 P1: pending_tokens 竞态修复 (`timer.rs:171-202`)

**问题**: cancel 和 register 之间存在竞态窗口

**修复**: `register()` 返回 `bool` 检查竞态状态
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

### 2.3 P1: TOCTOU openat 验证 (`lifecycle.rs:38-178`)

**问题**: 传统 symlink_metadata 检查存在 TOCTOU 窗口

**修复**: 添加 `openat_verifier` 模块（Unix）
```rust
pub fn verify_path_no_symlink(base_dir: &Path, relative_path: &str) -> Result<(), LifecycleError> {
    let base_fd = libc::open(base_dir, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
    for component in dirs {
        let next_fd = libc::openat(current_fd, component, O_NOFOLLOW | O_DIRECTORY);
    }
    // fstatat 检查最终文件
}
```

**Codex 指出的剩余问题**: openat 只验证不安全打开，entry_path() 返回路径后仍按路径打开

### 2.4 P2: 回归测试 (9 个新测试)

| 测试 | 文件 | 说明 |
|------|------|------|
| `test_cancel_before_register_race` | timer.rs | register 前取消 |
| `test_cancel_during_register_window` | timer.rs | 并发竞态测试 |
| `test_recover_lock_normal` | timer.rs | 正常锁恢复 |
| `test_recover_lock_poisoned` | timer.rs | 毒化锁恢复 |
| `test_openat_verifier_normal_path` | lifecycle.rs | 正常路径通过 |
| `test_openat_verifier_rejects_symlink` | lifecycle.rs | symlink 目录拒绝 |
| `test_openat_verifier_rejects_symlink_file` | lifecycle.rs | symlink 文件拒绝 |
| `test_entry_path_rejects_absolute_path` | lifecycle.rs | 绝对路径拒绝 |
| `test_entry_path_rejects_path_traversal` | lifecycle.rs | 路径遍历拒绝 |

---

## 3. 验证结果

```
cargo check: ✅ 通过（仅警告）
cargo test:  ✅ 84 passed
```

---

## 4. Codex 复审结果

**评分**: 77/100（上次 68 → +9 分）

| 维度 | 得分 | 变化 |
|------|------|------|
| 安全性 | 31/40 | +5 |
| 代码质量 | 22/30 | +2 |
| 架构设计 | 24/30 | +2 |

**评分趋势**:
```
初审 (v1): 52/100 ██████████░░░░░░░░░░
复审 (v2): 68/100 █████████████░░░░░░░
三审 (v3): 77/100 ███████████████░░░░░
目标:      80/100 ████████████████░░░░
```

---

## 5. 剩余问题（Codex 发现）

### P1 级别

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **TOCTOU 窗口仍存在** | openat 只验证不安全打开，entry_path() 返回路径后仍按路径打开 | `lifecycle.rs:59,512`, `runtime.rs:685,709` |
| **Timer 竞态窗口** | register() 先删 pending 再 await 获取 timers 锁，cancel 可能两表都找不到 | `timer.rs:180,194,210` |

### P2 级别

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| new_with_fallback panic | 双重失败时仍 panic | `fetch.rs:433,457` |
| 测试恒真断言 | test_cancel_during_register_window 断言恒真 | `timer.rs:534` |
| 冗余校验 | starts_with 检查恒真 | `lifecycle.rs:521,529` |

---

## 6. 后续任务

### 必须修复（达到 80 分）

1. **TOCTOU 彻底修复**
   - openat 链式打开返回 `OwnedFd`/`File`
   - 用 fd 读取文件，而非路径
   - 消除"验证-使用"窗口

2. **Timer 竞态彻底修复**
   - 方案 A：合并 pending_tokens/timers 为单一状态表
   - 方案 B：cancel 即使命中 pending 也继续检查 timers

3. **移除 panic**
   - new_with_fallback 改 Result 或禁用 fetch 能力

4. **修复测试**
   - 删除恒真断言
   - 添加定向竞态测试

---

## 7. 产出文件

| 文件 | 说明 |
|------|------|
| `timer.rs` | +70 行（recover_lock + 竞态修复 + 测试） |
| `lifecycle.rs` | +190 行（openat_verifier + 测试 + Default derive） |
| `.claude/review-report-phase2-v3.md` | Codex 三审报告 (77分) |
| `.claude/catchup/session-2025-12-28-security-review.md` | 修复会话总结 |

---

## 8. 关键决策记录

1. **Mutex 恢复策略**: 使用 `recover_lock()` 恢复毒化锁数据，避免连锁 panic
2. **竞态检测策略**: register() 返回 bool 检查 pending 状态
3. **TOCTOU 方案**: Unix 使用 openat 链式验证，Windows fallback 到 symlink_metadata
4. **测试策略**: 使用并发 tokio::join! 测试竞态

---

**会话总结生成时间**: 2025-12-28T22:15:00+08:00
