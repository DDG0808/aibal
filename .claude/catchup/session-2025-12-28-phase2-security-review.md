# 会话总结: Phase 2 插件运行时核心安全审查与修复

**日期**: 2025-12-28
**会话类型**: 代码审查 + 安全修复
**范围**: `src-tauri/src/plugin/` 目录

---

## 1. 会话目标

1. 完成 `ipc.rs` 中所有 TODO 功能实现
2. 使用 Codex 深度审查 Phase 2 插件运行时核心
3. 修复 Codex 审查发现的安全和稳定性问题

---

## 2. 完成的工作

### 2.1 IPC Commands 实现

**修改文件**:
- `src/plugin/lifecycle.rs`
- `src/commands/ipc.rs`

**新增功能**:
| 命令 | 状态 | 说明 |
|------|------|------|
| `plugin_uninstall` | ✅ | 卸载插件并删除目录 |
| `plugin_reload` | ✅ | 重载插件 manifest |
| `plugin_update` | ✅ | 以 reload 实现 |
| `get_plugin_config` | ✅ | 获取插件配置 |
| `set_plugin_config` | ✅ | 设置插件配置 |
| `validate_plugin_config` | ✅ | 验证插件配置 |
| `get_all_health` | ✅ | 获取所有插件健康状态 |
| `get_plugin_health` | ✅ | 获取单个插件健康状态 |
| `get_all_data` | ✅ | 获取所有插件数据 |
| `get_plugin_data` | ✅ | 获取单个插件数据 |
| `refresh_plugin` | ✅ | 刷新单个插件 |
| `refresh_all` | ✅ | 刷新所有插件 |

**PluginInstance 扩展**:
- 添加 `config: HashMap<String, Value>` 字段
- 添加 `cached_data: Option<PluginData>` 字段
- 添加健康统计字段：`last_success`、`error_count`、`success_count`、`total_latency_ms`
- 新增方法：`to_health()`、`record_success()`、`record_failure()`

---

### 2.2 Codex 初审结果

**评分**: 52/100
- 安全性: 21/40
- 代码质量: 16/30
- 架构设计: 15/30

**发现的问题**:

| 级别 | 问题 | 文件 |
|------|------|------|
| P0 | 沙盒默认不生效 | `runtime.rs:242` |
| P0 | 隔离面不足（Function 未移除） | `sandbox/mod.rs:52` |
| P0 | 权限系统未落地 | `lifecycle.rs:89` |
| P0 | 路径遍历风险 | `lifecycle.rs:313` |
| P1 | DNS 解析无超时 | `fetch.rs:790` |
| P1 | HTTP fallback fail-open | `fetch.rs:452` |
| P1 | watcher blocking_send | `watcher.rs:54` |
| P1 | 同步 IO 在 async 中 | `lifecycle.rs:567` |

---

### 2.3 安全修复

#### P0 修复

**1. 沙盒默认不生效** (`runtime.rs`)
```rust
// 新增方法，强制初始化沙盒
pub async fn create_sandboxed_context(&self) -> Result<AsyncContext, RuntimeError>

// 新增带权限的沙盒创建
pub async fn create_sandboxed_context_with_permissions(
    &self,
    permissions: &[String],
    request_manager: Option<Arc<RequestManager>>,
    timer_registry: Option<Arc<TimerRegistry>>,
) -> Result<AsyncContext, RuntimeError>
```

**2. 隔离面不足** (`sandbox/mod.rs:52`)
```rust
// 取消注释，移除 Function 构造器
let _ = globals.remove::<&str>("Function");
```

**3. 路径遍历风险** (`lifecycle.rs:323-384`)
```rust
// 重写 entry_path() 添加完整校验
pub fn entry_path(&self) -> Result<PathBuf, LifecycleError> {
    // 1. 检查绝对路径
    // 2. 检查 .. 组件
    // 3. 验证路径在插件目录内
}
```

#### P1 修复

**1. DNS 解析超时** (`fetch.rs:70,795-804`)
```rust
const DNS_TIMEOUT: Duration = Duration::from_secs(5);

// 使用 tokio::time::timeout 包装
tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host(&lookup_addr))
```

**2. HTTP fallback** (`fetch.rs:433-465`)
```rust
// 移除不安全的第二级 fallback
// 失败时 panic 而非降级到不安全客户端
```

**3. watcher 非阻塞** (`watcher.rs:54-66`)
```rust
// 改用 try_send 替代 blocking_send
if let Err(e) = tx.try_send(hot_event) {
    // 通道满时丢弃事件
}
```

**4. 异步 IO** (`lifecycle.rs:643-707`)
```rust
// 使用 tokio::fs 替代 std::fs
tokio::fs::remove_dir_all(&path).await
tokio::fs::read_to_string(&manifest_path).await
```

---

### 2.4 Codex 复审结果

**评分**: 68/100 (+16)
- 安全性: 26/40 (+5)
- 代码质量: 20/30 (+4)
- 架构设计: 22/30 (+7)

**剩余问题**:

| 级别 | 问题 | 说明 |
|------|------|------|
| P0 | Function 构造器可绕过 | `(function(){}).constructor` 仍可获取 |
| P0 | 修复未被调用 | `create_sandboxed_context*` 定义但无调用点 |
| P0 | symlink 逃逸 | `entry_path` 未检查 symlink |
| P1 | discover() 仍同步 | 使用 `std::fs::read_dir` |
| P1 | Timer 资源风险 | spawn 前未检查数量限制 |

---

## 3. 产出文件

| 文件 | 说明 |
|------|------|
| `.claude/review-report-phase2.md` | Codex 初审报告 (52分) |
| `.claude/review-report-phase2-v2.md` | Codex 复审报告 (68分) |
| `.claude/context-initial.json` | Auggie 搜索结果缓存 |

---

## 4. 后续任务

### 待完成（P0）

1. **彻底禁用 Function 构造器**
   - 冻结 `Function.prototype.constructor`

2. **确保沙盒入口被调用**
   - 在 `PluginManager` 中添加执行插件的方法
   - 强制使用安全的 context 创建方法

3. **entry_path 添加 symlink 检查**
   - 使用 `symlink_metadata` 检测

### 待完成（P1）

1. **discover() 改异步**
   - 使用 `tokio::fs::read_dir`

2. **Timer API 前置检查**
   - 在 spawn 前检查数量限制

---

## 5. 评分趋势

```
初审 (v1): 52/100 ██████████░░░░░░░░░░
复审 (v2): 68/100 █████████████░░░░░░░
目标:      80/100 ████████████████░░░░
```

---

## 6. 关键决策记录

1. **HTTP fallback 策略**: 选择 panic 而非降级到不安全客户端（安全优先）
2. **watcher 丢弃策略**: 通道满时丢弃事件而非阻塞（稳定性优先）
3. **entry_path 返回 Result**: 改变签名以支持安全校验

---

**会话总结生成时间**: 2025-12-28T16:35:00+08:00
