# Codex 代码审查报告: Phase 2 插件运行时核心

**日期**: 2025-12-28
**审查范围**: `src-tauri/src/plugin/` 目录
**模型**: Codex (推理强度: xhigh)
**耗时**: 855 秒

---

## 1. 综合评分：52 / 100

| 维度 | 得分 | 满分 |
|------|------|------|
| 安全性 | 21 | 40 |
| 代码质量 | 16 | 30 |
| 架构设计 | 15 | 30 |

---

## 2. 各维度详细分析

### 安全性（21/40）

**正向点**
- 运行时资源限制思路完整：内存/栈限制 + interrupt_handler + Watchdog 超时中断（`runtime.rs:25`、`runtime.rs:209`、`runtime.rs:215`、`runtime.rs:372`）
- `fetch` 的 SSRF 防护设计质量较高：scheme 限制、私网/特殊网段识别、DNS rebinding 检测、`resolve()` 固定 IP、禁用 redirect/proxy、响应体流式大小限制、RAII 并发槽位（`fetch.rs:631`、`fetch.rs:724`、`fetch.rs:776`、`fetch.rs:241`、`fetch.rs:287`、`fetch.rs:579`）
- 宿主侧额外防护意识到位：console 输出截断 + 深度限制（`console.rs:9`、`console.rs:99`），encoding 输入 1MB 限制避免绕过 QuickJS 内存限制（`encoding.rs:13`、`encoding.rs:40`、`encoding.rs:106`）

**主要风险/缺口**
- **沙盒"默认不生效/可被误用"**：`SandboxRuntime::create_context()` 直接返回 `AsyncContext::full`，且仓库内未发现任何调用 `SandboxApiInitializer::init_basic/init_with_fetch` 的代码路径（`runtime.rs:242`；`sandbox/mod.rs:62`；`sandbox/mod.rs:79`）。这意味着一旦上层忘记显式初始化，所谓"移除危险全局对象/只暴露安全 API"不会发生
- **隔离面不足**：仅移除了 `eval`，但注释中提到的 `Function` 构造器默认未移除（可用于动态代码生成）（`sandbox/mod.rs:47`、`sandbox/mod.rs:50`）
- **权限系统未落地**：manifest 声明了 `permissions`，但全项目没有任何使用点，无法形成"默认拒绝/按需开放"的安全边界（`lifecycle.rs:89`）
- **路径遍历风险（预埋）**：入口文件路径由 `self.path.join(self.manifest.entry)` 直接拼接，未限制 `entry` 必须为相对且无 `..`，后续若用于读取/执行会产生目录逃逸风险（`lifecycle.rs:313`）
- **DNS 阶段 DoS 风险**：`tokio::net::lookup_host` 未包 `timeout`，在恶意域名/异常解析器环境下可能长时间悬挂，占用并发槽位（`fetch.rs:790`）
- **安全降级策略偏"fail-open"**：`RequestManager::new_with_fallback` 最终 fallback 到 `reqwest::Client::new()`，可能重新启用 proxy/redirect 等默认行为（`fetch.rs:435`、`fetch.rs:452`、`fetch.rs:458`）
- 备注：当前 JS 侧 `fetch` 实际是"直接抛错不可用"，短期降低 SSRF 面，但也容易让安全评审误判（`fetch.rs:156`、`fetch.rs:170`）

### 代码质量（16/30）

**正向点**
- 错误类型可读性较好：`RuntimeError`/`FetchError`/`LifecycleError`（`runtime.rs:41`、`fetch.rs:21`、`lifecycle.rs:27`）
- 并发计数与释放做得细：CAS 自旋 + `fetch_update` 防下溢 + RAII `Drop` 保证释放（`fetch.rs:474`、`fetch.rs:500`、`fetch.rs:592`）

**主要问题**
- 多处"对外导出但实现为占位/示例"，容易误导调用方：`memory_usage()` 永远返回 0（`runtime.rs:272`），`Executor::execute_with_timeout` 标注"框架示例"且不在 `context.with` 中执行 JS（`runtime.rs:445`、`runtime.rs:464`）
- Timer API 逻辑未完成：`setTimeout/setInterval` 不执行 JS 回调，仅日志；且 `callback` 未被捕获持久化（`timer.rs:204`、`timer.rs:239`、`timer.rs:288`）
- watcher 回调线程使用 `blocking_send`，在事件风暴/通道满时可能阻塞 notify 线程（`watcher.rs:54`、`watcher.rs:58`）；`debounce_ms` 字段未真正参与处理（`watcher.rs:145`、`watcher.rs:156`）
- 生命周期管理中在 async 方法持锁期间执行同步 IO（如 `remove_dir_all`），存在阻塞 Tokio worker 的风险（`lifecycle.rs:567`、`lifecycle.rs:577`）

### 架构设计（15/30）

**正向点**
- 模块拆分清晰：lifecycle / runtime / sandbox / watcher / types（`mod.rs:4`）

**核心缺口**
- 缺少"装配层/单一入口"保证：runtime 创建 context → 注入 sandbox API → 根据 manifest.permissions 裁剪能力 → 执行/卸载时回收资源（Timer/Request/Subscription）这一条链路目前未打通（`runtime.rs:241`；`sandbox/mod.rs:58`；`lifecycle.rs:175`）
- ResourceRegistry/TimerRegistry/RequestManager 各自存在，但未形成统一的"插件卸载强制回收"闭环（`lifecycle.rs:175`；`timer.rs:103`；`fetch.rs:388`）
- 与 Tauri IPC 的集成点在本范围内不可见（意味着可能尚未落地；至少 `src/plugin/` 中未体现）

---

## 3. 具体问题列表

| 问题 | 文件:行号 | 严重性 |
|------|-----------|--------|
| 沙盒默认不生效：context 创建未强制初始化 sandbox | `runtime.rs:242`；`sandbox/mod.rs:62` | 🔴 高 |
| 沙盒隔离不足：仅移除 `eval`，`Function` 仍可用 | `sandbox/mod.rs:47`、`sandbox/mod.rs:50` | 🔴 高 |
| 权限声明未执行：`permissions` 只在 manifest 定义，无 enforcement | `lifecycle.rs:89` | 🔴 高 |
| 潜在路径遍历：`entry_path` 未校验 `manifest.entry` | `lifecycle.rs:313` | 🔴 高 |
| 卸载删除目录缺少路径边界/安全删除策略 | `lifecycle.rs:567`、`lifecycle.rs:577` | 🟠 中 |
| DNS 解析无超时：可能导致挂起/DoS | `fetch.rs:790` | 🟠 中 |
| HTTP client 安全降级可能 fail-open：fallback 到 `Client::new()` | `fetch.rs:435`、`fetch.rs:458` | 🟠 中 |
| JS `fetch` 目前为"抛错模式"，与对外 API 预期不一致 | `fetch.rs:156`、`fetch.rs:171` | 🟡 低 |
| Timer API 不执行回调（功能缺失 + 资源回收闭环难验证） | `timer.rs:239`、`timer.rs:291` | 🟠 中 |
| watcher 使用 `blocking_send` 有阻塞风险，且 `debounce_ms` 未实际生效 | `watcher.rs:54`、`watcher.rs:145` | 🟠 中 |
| `memory_usage()` 为 TODO 返回 0，监控/限额缺失 | `runtime.rs:272` | 🟡 低 |
| `Executor::execute_with_timeout` 为示例实现，可能被误用 | `runtime.rs:445`、`runtime.rs:464` | 🟡 低 |

---

## 4. 优先级排序的修复建议

### P0（先补安全边界，避免"看似沙盒实际裸奔"）

1. **让沙盒初始化成为"默认必经路径"**
   - 提供 `create_sandboxed_context()`（或类似）在内部调用 `SandboxApiInitializer::init_basic/init_with_fetch`
   - 在所有执行入口只暴露该方法

2. **明确并收敛 QuickJS 能力面**
   - 避免直接使用 `AsyncContext::full` 作为默认
   - 至少在装配层中显式移除/禁用潜在危险能力
   - 补齐 `Function` 是否允许的决策（`sandbox/mod.rs:50`）

3. **落地 `manifest.permissions`**
   - 按权限决定是否注入 `fetch/timer/…`（默认拒绝）
   - 在运行时对敏感 API 做二次校验（防止绕过注入层）

4. **校验 `manifest.entry` 与所有插件可控路径**
   - 必须相对路径、禁止 `..`、禁止绝对路径
   - 必要时 canonicalize 并校验前缀

### P1（补稳定性与 SSRF 细节）

1. 给 `lookup_host` 增加 `tokio::time::timeout`，并在错误信息中区分解析超时/失败（`fetch.rs:790`）
2. 移除 `RequestManager` 的不安全 fallback，或确保 fallback 仍保持 `redirect none + no_proxy + timeout`（`fetch.rs:452`）
3. watcher 改为非阻塞投递（`try_send`/丢弃策略/独立线程队列），并真正实现 debounce（`watcher.rs:54`、`watcher.rs:145`）
4. 生命周期管理中的重 IO（读 manifest、删目录）改为 `tokio::fs` 或 `spawn_blocking`，避免阻塞 async worker（`lifecycle.rs:577`）

### P2（清理误导性 API 与补齐实现）

1. 补齐或移除 `Executor::execute_with_timeout` / `SandboxRuntime::memory_usage` 的占位实现，避免外部调用误判（`runtime.rs:272`、`runtime.rs:445`）
2. Timer API 补齐"在 JS 线程执行回调"的调度机制，并在插件卸载时统一 `cancel_all`（`timer.rs:239`）
3. 为"沙盒初始化必须发生/权限裁剪必须生效/路径校验必须阻断"补充单测或集成测试用例（本阶段已有测试框架可复用：`tests.rs:10`）

---

## 5. 后续行动

- [ ] P0 安全边界修复（预估工作量：大）
- [ ] P1 稳定性修复（预估工作量：中）
- [ ] P2 API 清理与测试补充（预估工作量：小）

---

**审查报告生成时间**: 2025-12-28T15:59:00+08:00
