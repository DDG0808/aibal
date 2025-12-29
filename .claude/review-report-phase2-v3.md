# Codex 代码审查报告: Phase 2 插件运行时核心（第三轮复审）

**日期**: 2025-12-28
**审查范围**: `src-tauri/src/plugin/` 目录
**模型**: Codex (推理强度: xhigh)
**耗时**: 877 秒

---

## 1. 综合评分：77 / 100

| 维度 | 得分 | 满分 | 变化 |
|------|------|------|------|
| 安全性 | 31 | 40 | +5 |
| 代码质量 | 22 | 30 | +2 |
| 架构设计 | 24 | 30 | +2 |
| **总计** | **77** | **100** | **+9** |

---

## 2. 评分趋势

```
初审 (v1): 52/100 ██████████░░░░░░░░░░
复审 (v2): 68/100 █████████████░░░░░░░
三审 (v3): 77/100 ███████████████░░░░░
目标:      80/100 ████████████████░░░░
```

---

## 3. 已确认的改进点

| 改进 | 文件:行号 |
|------|-----------|
| Function/eval/WebAssembly 禁用 + fail-close 自检 | `sandbox/mod.rs:54`, `sandbox/mod.rs:223` |
| SSRF/DNS rebinding 防护 (resolve + 禁 redirect/proxy) | `fetch.rs:206`, `fetch.rs:260`, `fetch.rs:782` |
| Timer 竞态修复 (register() 返回 bool) | `timer.rs:171`, `timer.rs:320` |
| Mutex 毒化锁恢复 | `timer.rs:31` |

---

## 4. 剩余问题列表

### P1 级别（高优先级）

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **TOCTOU 窗口仍存在** | `openat_verifier` 仅验证不安全打开，`entry_path()` 返回路径后仍按路径打开；`read_file_nofollow` 仅对最终组件用 O_NOFOLLOW | `lifecycle.rs:59,512`, `runtime.rs:685,709` |
| **Timer 竞态窗口** | `register()` 先删 pending 再 await 获取 timers 锁，期间 `cancel()` 可能两表都找不到 id | `timer.rs:180,194,210` |

### P2 级别（中优先级）

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **new_with_fallback panic** | 双重构建失败时 `panic!`，与注释矛盾 | `fetch.rs:433,457` |
| **测试恒真断言** | `test_cancel_during_register_window` 的断言恒真 | `timer.rs:534` |
| **冗余校验** | `resolved.starts_with(plugin_dir)` 基本恒真 | `lifecycle.rs:521,529` |
| **JS fetch 未对接** | 当前抛错模式，安全实现在 `secure_fetch` | `fetch.rs:176` |

---

## 5. 具体改进建议

### P1 TOCTOU 彻底修复

```rust
// 方案：openat 链式打开返回 OwnedFd，用于后续读取
pub fn open_entry_file_safe(base_dir: &Path, entry: &str) -> Result<OwnedFd, LifecycleError> {
    // 链式 openat 打开所有中间目录
    let mut current_fd = open(base_dir, O_RDONLY | O_DIRECTORY | O_NOFOLLOW)?;
    for component in Path::new(entry).parent().unwrap().components() {
        current_fd = openat(current_fd, component, O_RDONLY | O_DIRECTORY | O_NOFOLLOW)?;
    }
    // 最终用 openat 打开文件
    let file_fd = openat(current_fd, entry.file_name(), O_RDONLY | O_NOFOLLOW)?;
    Ok(file_fd)
}

// 调用方使用 fd 读取，而非路径
let fd = open_entry_file_safe(&plugin_dir, entry)?;
let content = read_fd_to_string(fd)?;
```

### P1 Timer 竞态修复

```rust
// 方案 A：合并为单一状态表
enum TimerState {
    Pending { cancel_token: CancellationToken },
    Registered { entry: TimerEntry },
}
timers: Mutex<HashMap<u64, TimerState>>

// 方案 B：cancel 即使命中 pending 也继续检查 timers
pub async fn cancel(&self, id: u64) -> bool {
    let found_in_pending = {
        let mut pending = recover_lock(self.pending_tokens.lock());
        if let Some(token) = pending.remove(&id) {
            token.cancel();
            true
        } else {
            false
        }
    };
    // 即使 pending 找到，也检查 timers（防止迁移窗口）
    let mut timers = self.timers.lock().await;
    if let Some(entry) = timers.remove(&id) {
        entry.cancel_token.cancel();
        true
    } else {
        found_in_pending
    }
}
```

### P2 移除 panic

```rust
pub fn new_with_fallback() -> Self {
    Self::new().unwrap_or_else(|e| {
        log::error!("HTTP 客户端创建失败，fetch 功能将不可用: {}", e);
        Self {
            client: None,  // 改为 Option<Client>
            active_requests: AtomicUsize::new(0),
        }
    })
}
```

---

## 6. 审查结论

**建议**: 退回（仍有 P1 级问题未完全收口）

**下一步**:
1. 修复 TOCTOU：openat 链式打开返回 fd/File
2. 修复 Timer 竞态：合并状态表或双重检查
3. 移除 panic
4. 修复测试恒真断言

---

**审查报告生成时间**: 2025-12-28T22:10:00+08:00
