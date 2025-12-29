# Codex 代码审查报告: Phase 2 插件运行时核心（修复后复审）

**日期**: 2025-12-28
**审查范围**: `src-tauri/src/plugin/` 目录
**模型**: Codex (推理强度: xhigh)
**耗时**: 839 秒

---

## 1. 综合评分：68 / 100

| 维度 | 得分 | 满分 | 变化 |
|------|------|------|------|
| 安全性 | 26 | 40 | +5 |
| 代码质量 | 20 | 30 | +4 |
| 架构设计 | 22 | 30 | +7 |
| **总计** | **68** | **100** | **+16** |

---

## 2. 与上次评分对比

**上次 52/100 → 本次 68/100（+16 分）**

### 主要加分点

1. **DNS 解析加超时并 fail-closed**（`fetch.rs:70`、`fetch.rs:793`）
2. **watcher 改非阻塞发送**（`watcher.rs:54`）
3. **部分 async 中的同步 IO 已替换为 `tokio::fs`**（`lifecycle.rs:643`、`lifecycle.rs:685`）
4. **新增沙盒化 context 创建入口**（`runtime.rs:257`、`runtime.rs:279`）
5. **移除全局 `eval/Function`**（`sandbox/mod.rs:43`）

---

## 3. 剩余问题列表

### P0 级别（高优先级）

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **沙盒隔离可绕过** | 仅删除全局 `Function` 不等于禁用 Function 构造器。JS 仍可通过 `(function(){}).constructor` 获取 Function 构造器 | `sandbox/mod.rs:43-55` |
| **修复落地缺少调用链证据** | `create_sandboxed_context*` 仅定义未被调用；`entry_path()` 同样未被使用 | `runtime.rs:257/279`、`lifecycle.rs:323` |
| **entry_path 未覆盖 symlink 逃逸** | 返回路径未对最终目标做 `canonicalize`/no-follow 校验 | `lifecycle.rs:323-384` |

### P1 级别（中优先级）

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **async 中仍存在阻塞 IO** | `discover_and_load` 调用同步的 `discover()`，内部使用 `std::fs::read_dir` | `lifecycle.rs:566-568`、`lifecycle.rs:508` |
| **Timer API 资源/DoS 风险** | 每次调用先 `tokio::spawn` 再做数量限制；回调执行未闭环 | `timer.rs:224-233`、`timer.rs:235-242` |

### P2 级别（低优先级）

| 问题 | 说明 | 文件:行号 |
|------|------|-----------|
| **RequestManager fallback 可能 panic** | 双重构建失败时直接 `panic!` | `fetch.rs:433-463` |
| **错误序列化质量** | `PluginError::toJSON` 实现可改进 | - |

---

## 4. 进一步优化建议

### P0 修复（必须）

1. **彻底禁用 Function 构造器**
   ```rust
   // 在 remove_dangerous_globals 中添加
   ctx.eval::<(), _>(r#"
       Object.defineProperty(Function.prototype, 'constructor', {
           get: function() { throw new Error('Function constructor is disabled'); },
           configurable: false
       });
   "#)?;
   ```

2. **确保沙盒入口被实际调用**
   - 在 `PluginManager` 中添加执行插件的方法
   - 强制使用 `create_sandboxed_context_with_permissions`

3. **entry_path 添加 symlink 检查**
   ```rust
   // 使用 std::fs::symlink_metadata 检查是否为 symlink
   if full_path.symlink_metadata()?.file_type().is_symlink() {
       return Err(LifecycleError::PluginLoad("symlink not allowed".into()));
   }
   ```

### P1 修复（建议）

1. **discover() 改为异步版本**
   - 使用 `tokio::fs::read_dir` 替代 `std::fs::read_dir`

2. **Timer API 前置数量检查**
   - 在 spawn 之前检查 timer 数量限制

---

## 5. 评分趋势

```
初审 (v1): 52/100 ██████████░░░░░░░░░░
复审 (v2): 68/100 █████████████░░░░░░░
目标:      80/100 ████████████████░░░░
```

---

**审查报告生成时间**: 2025-12-28T16:29:00+08:00
