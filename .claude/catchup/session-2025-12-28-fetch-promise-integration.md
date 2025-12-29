# 会话总结: JS Fetch Promise 集成尝试

**日期**: 2025-12-28
**上下文来源**: `session-2025-12-28-architecture-fixes-final.md`
**目标**: 实现 JS fetch Promise 异步集成，将 Codex 评分从 91 提升至 95+

---

## 完成状态

### 已完成
1. **context7 文档研究**: 获取了 rquickjs 0.10.0 的 Async 函数和 Promise API 文档
2. **SandboxApiInitializer 更新**: 添加了 `init_with_fetch` 方法，支持 fetch API 注入
3. **编译验证**: `cargo check` 和 `cargo test` 均通过（74 个测试全部通过）

### 遇到的技术障碍

**rquickjs 0.6 生命周期限制**

尝试实现 `Promise::new + ctx.spawn` 模式时遇到不可解决的生命周期错误：

```rust
error: lifetime may not live long enough
   --> src/plugin/sandbox/fetch.rs:184:17
    |
159 |             Function::new(ctx.clone(), move |ctx: Ctx<'_>, url: String| {
    |                                              ---                      - return type
    |                                              |
    |                                              has type `rquickjs::Ctx<'1>`
...
184 |                 Ok(promise)
    |                 ^^^^^^^^^^^ returning this value requires that `'1` must outlive `'2`
```

**根本原因分析**:
1. `Function::new` 接受 `Ctx<'js>`，闭包必须满足 `IntoJsFunc<'js, P> + 'js`
2. `move` 闭包捕获 `Arc<RequestManager>` 后，返回的 `Promise<'_>` 生命周期无法正确推断
3. rquickjs 0.6.2 的 API 与 0.10.0 文档示例存在差异

**尝试过的解决方案**:
- `Async` 包装器 → 同样的生命周期问题
- 显式返回类型 `JsResult<Promise<'_>>` → 编译失败
- 使用 `Value` 替代 `Promise` → 同样失败
- Turbofish 语法明确类型 → 无法解决生命周期问题

---

## 当前实现状态

`FetchApi::inject` 保持同步占位符实现：

```rust
// fetch.rs:155-179
globals.set(
    "fetch",
    Function::new(ctx.clone(), |ctx: Ctx<'_>, url: String| -> JsResult<String> {
        // 同步验证 URL（安全检查）
        if let Err(e) = UrlSecurityChecker::check_url(&url) {
            return Err(Exception::throw_type(&ctx, &e.to_string()));
        }

        // 返回占位符 JSON
        log::debug!("Fetch 请求已创建（同步占位）: {}", url);
        Ok(format!(
            r#"{{"url":"{}","method":"GET","pending":true,"note":"async_integration_pending"}}"#,
            url.replace('"', "\\\"")
        ))
    })?,
)?;
```

**安全特性已实现**:
- URL 模式检查（禁止 localhost、私有 IP、内部域名）
- 输入验证和错误处理

---

## 后续解决方案（推荐）

### 方案 1: 升级 rquickjs 到 0.10+
```toml
# Cargo.toml
rquickjs = { version = "0.10", features = [...] }
```
优点: API 更完善，文档示例可直接使用
缺点: 可能有 breaking changes

### 方案 2: 使用 Persistent 包装器
```rust
use rquickjs::Persistent;

let resolve_persistent = Persistent::save(ctx, resolve);
let reject_persistent = Persistent::save(ctx, reject);

tokio::spawn(async move {
    // 在异步任务中恢复
    let resolve = resolve_persistent.restore(ctx)?;
    resolve.call::<_, ()>((result,))?;
});
```
优点: 解决生命周期问题
缺点: 需要额外的上下文传递机制

### 方案 3: 全局 RequestManager 静态引用
```rust
static REQUEST_MANAGER: OnceCell<Arc<RequestManager>> = OnceCell::new();
```
优点: 避免闭包捕获
缺点: 引入全局状态，不够优雅

---

## 文件变更清单

| 文件 | 变更 | 说明 |
|------|------|------|
| `fetch.rs:15` | 修改 | 导入更新 |
| `fetch.rs:137-190` | 修改 | inject 方法文档和实现 |
| `mod.rs:19` | 添加 | `use std::sync::Arc` |
| `mod.rs:47-74` | 添加 | `init_with_fetch` 方法 |

---

## 测试结果

```
running 74 tests
...
test result: ok. 74 passed; 0 failed; 0 ignored
```

---

## 待解决

1. **JS fetch Promise 异步集成**（优先级：高）
   - 需要选择上述方案之一
   - 或等待 rquickjs 升级

2. **Codex 审核**
   - 当前评分预计：91（无变化）
   - 需完成异步集成后重新审核

---

## 参考资料

- rquickjs 0.10.0 文档: `ctx.spawn` + `Promise::new`
- 设计文档: `插件系统设计文档.md` → fetch API 实现
- 上一会话总结: `session-2025-12-28-architecture-fixes-final.md`

---

**会话总结生成时间**: 2025-12-28 15:20 UTC+8
