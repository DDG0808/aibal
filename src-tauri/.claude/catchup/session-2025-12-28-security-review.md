# Phase 2 插件运行时安全审查会话总结

**日期**: 2025-12-28
**会话类型**: 安全代码审查与修复
**审查工具**: Codex (GPT-5.2, xhigh reasoning)

---

## 一、会话目标

对 Phase 2 插件运行时核心代码进行深度安全审查，修复所有 P0/P1 级别问题。

### 核心审查文件

| 文件 | 职责 |
|:--|:--|
| `src/plugin/runtime.rs` | 沙盒运行时、PluginExecutor、Watchdog |
| `src/plugin/sandbox/mod.rs` | 沙盒 API 初始化、Function/WebAssembly 禁用 |
| `src/plugin/sandbox/timer.rs` | 定时器 API、资源限制 |
| `src/plugin/sandbox/fetch.rs` | 网络请求、SSRF/DNS rebinding 防护 |
| `src/plugin/lifecycle.rs` | 插件生命周期、路径安全 |

---

## 二、审查轮次与评分

| 轮次 | 评分 | 变化 | 主要发现/修复 |
|:--|:--|:--|:--|
| 1 | 52/100 | - | 初始审查，发现大量安全问题 |
| 2 | 68/100 | +16 | 沙盒基础、SSRF 防护、权限注入 |
| 3 | 62/100 | -6 | 发现新问题：Function 原型链绕过、PluginExecutor 未调用 |
| 4 | 79/100 | +17 | 自检 fail-close、Semaphore、O_NOFOLLOW |
| 5 | 74/100 | -5 | 发现新问题：Timer 资源泄漏 |
| 6 | 预期 85+ | +11 | 完整资源释放、WebAssembly 禁用 |

---

## 三、已完成的安全修复

### 3.1 P0 级别（必须修复）

#### 1. Function 构造器禁用 + 自检 fail-close
**文件**: `src/plugin/sandbox/mod.rs`

**问题**:
- 仅删除 `globalThis.eval/Function` 不够，可通过原型链绕过
- 缺少 AsyncGeneratorFunction 处理
- 删除失败时静默放行

**修复**:
```javascript
// 禁用 4 种构造器原型链
disableConstructor(FunctionConstructor, 'Function');
disableConstructor(AsyncFunctionConstructor, 'AsyncFunction');
disableConstructor(GeneratorFunctionConstructor, 'GeneratorFunction');
disableConstructor(AsyncGeneratorFunctionConstructor, 'AsyncGeneratorFunction');

// 自检 + fail-close
var errors = [];
try { globalThis.eval; errors.push('eval accessible'); }
catch(e) { if (!(e instanceof TypeError)) errors.push('wrong error'); }
// ... 检查所有 7 个入口点 ...
if (errors.length > 0) throw new Error('Sandbox security check failed');
```

#### 2. WebAssembly 禁用
**文件**: `src/plugin/sandbox/mod.rs`

**问题**: WebAssembly 是另一个动态代码执行入口

**修复**:
```javascript
if (typeof globalThis.WebAssembly !== 'undefined') {
    disableProperty(globalThis, 'WebAssembly', 'WebAssembly is disabled');
}
// + 自检验证
```

#### 3. PluginExecutor 超时保护
**文件**: `src/plugin/runtime.rs`

**问题**:
- PluginExecutor 未使用 run_with_limits()
- 直接 ctx.with() 无超时保护

**修复**:
```rust
pub async fn execute_plugin(&self, code: &str, permissions: &[String]) -> Result<...> {
    let ctx = self.runtime.create_sandboxed_context_with_permissions(...).await?;
    // 使用 run_with_limits 启用超时保护
    let json_str = self.runtime.run_with_limits(&ctx, move |js_ctx| {
        let result = js_ctx.eval(code)?;
        // ...
    }).await?;
}
```

#### 4. Timer 资源泄漏修复
**文件**: `src/plugin/sandbox/timer.rs`

**问题**:
- cancel 分支未调用 complete()
- permit 和 entry 永久保留

**修复**:
```rust
tokio::spawn(async move {
    // 先检查是否已取消
    if cancel_token.is_cancelled() {
        return;  // permit 自动释放
    }
    registry_clone.register(...).await;
    tokio::select! {
        _ = cancel_token_clone.cancelled() => {
            registry_clone.complete(id).await;  // 添加清理
        }
        _ = sleep(delay) => {
            registry_clone.complete(id).await;
        }
    }
});
```

### 3.2 P1 级别（高优先）

#### 1. Timer Semaphore 真正占位
**问题**: reserve_slot_sync 只发号不占位

**修复**: 使用 `tokio::sync::Semaphore` + `OwnedSemaphorePermit`
```rust
pub fn try_acquire(&self) -> Option<(u64, OwnedSemaphorePermit, CancellationToken)> {
    let permit = self.semaphore.clone().try_acquire_owned().ok()?;
    // permit 持有期间真正占用槽位
}
```

#### 2. Timer 取消竞态
**问题**: cancel 在 register 前调用无效

**修复**: 添加 `pending_tokens` 同步 map
```rust
pub struct TimerRegistry {
    pending_tokens: std::sync::Mutex<HashMap<u64, CancellationToken>>,
    timers: Mutex<HashMap<u64, TimerEntry>>,
}
```

#### 3. Watchdog biased select
**问题**: stop 与 sleep 同时 ready 时可能误触发

**修复**:
```rust
tokio::select! {
    biased;  // 优先检查 stop 信号
    _ = stop_rx => { ... }
    _ = sleep(timeout) => { ... }
}
```

#### 4. O_NOFOLLOW 防止 TOCTOU
**文件**: `src/plugin/runtime.rs`

**修复**:
```rust
#[cfg(unix)]
async fn read_file_nofollow(path: &Path) -> Result<String, RuntimeError> {
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
}
```

#### 5. TypeError 校验统一
**问题**: 部分构造器未验证错误类型

**修复**: 所有 7 个入口点都检查 `instanceof TypeError`

### 3.3 P2 级别（改进项）

| 修复项 | 说明 |
|:--|:--|
| 移除未使用变量 | `let start = Instant::now()` |
| Watchdog 注释修正 | 明确"无法中断阻塞宿主函数"限制 |

---

## 四、新增依赖

| 依赖 | 版本 | 用途 |
|:--|:--|:--|
| `tokio-util` | 0.7 | CancellationToken |
| `libc` | 0.2 | O_NOFOLLOW (Unix) |

---

## 五、剩余问题（待后续处理）

### P1 级别

| 问题 | 说明 | 建议方案 |
|:--|:--|:--|
| pending_tokens 窗口期 | register/cancel 之间可能漏取消 | 使用原子操作或更细粒度锁 |
| 中间目录 TOCTOU | O_NOFOLLOW 只覆盖最终组件 | 使用 openat + 逐段 nofollow |
| 阻塞宿主函数超时 | Watchdog 无法强制取消 | 宿主 API 内置超时机制 |

### P2 级别

| 问题 | 说明 |
|:--|:--|
| std::sync::Mutex unwrap() | poison 时会 panic |
| 缺少回归测试 | 尤其是竞态场景 |
| loader feature | 需明确模块加载边界 |

---

## 六、关键代码位置

### 安全入口
- `SandboxApiInitializer::remove_dangerous_globals()` - `sandbox/mod.rs:51`
- `PluginExecutor::execute_plugin()` - `runtime.rs:621`
- `read_file_nofollow()` - `runtime.rs:692`

### 资源管理
- `TimerRegistry::try_acquire()` - `timer.rs:129`
- `TimerRegistry::cancel()` - `timer.rs:177`
- `Watchdog::start()` - `runtime.rs:481`

---

## 七、验证命令

```bash
# 编译检查
cargo check

# 运行测试
cargo test

# 安全审计
cargo audit
```

---

## 八、后续建议

1. **添加回归测试**: 覆盖 Timer 竞态、自检失败等场景
2. **完善 TOCTOU 防护**: 使用 openat 逐段验证
3. **宿主 API 超时**: 所有可能阻塞的 API 添加内部超时
4. **定期 Codex 审核**: 每次重大修改后运行审核

---

## 九、参考资料

- [PortSwigger: Attacking JavaScript Sandboxes](https://portswigger.net/research/attacking-and-defending-javascript-sandboxes)
- [SEI CERT: Avoid TOCTOU Race Conditions](https://wiki.sei.cmu.edu/confluence/display/c/POS35-C)
- [Tokio: Limiting Spawned Tasks with Semaphore](https://users.rust-lang.org/t/can-tokio-semaphore-be-used-to-limit-spawned-tasks/59899)
