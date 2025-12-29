// QuickJS 运行时管理模块
// Phase 2.1: QuickJS 集成
//
// 实现任务:
// - 2.1.1 rquickjs 依赖 ✓
// - 2.1.2 AsyncRuntime 创建
// - 2.1.3 AsyncContext 创建
// - 2.1.4 内存限制 (16MB)
// - 2.1.5 栈大小限制 (512KB)
// - 2.1.6 interrupt_handler (CPU 超时中断)
// - 2.1.7 watchdog 任务

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rquickjs::{AsyncContext, AsyncRuntime, Error as JsError};
use thiserror::Error;
use tokio::sync::oneshot;

use crate::plugin::sandbox::{RequestManager, SandboxApiInitializer, TimerApi, TimerRegistry};

// ============================================================================
// 常量定义
// ============================================================================

/// 默认内存限制: 16MB
pub const DEFAULT_MEMORY_LIMIT: usize = 16 * 1024 * 1024;

/// 默认栈大小限制: 512KB
pub const DEFAULT_STACK_SIZE: usize = 512 * 1024;

/// 默认执行超时: 30 秒
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

/// interrupt_handler 检查间隔 (每 N 次操作检查一次)
const INTERRUPT_CHECK_INTERVAL: u64 = 10000;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("运行时创建失败: {0}")]
    RuntimeCreation(String),

    #[error("上下文创建失败: {0}")]
    ContextCreation(String),

    #[error("执行超时: 超过 {0:?}")]
    ExecutionTimeout(Duration),

    #[error("内存超限: 超过 {0} 字节")]
    MemoryExceeded(usize),

    #[error("执行被中断")]
    Interrupted,

    #[error("JS 执行错误: {0}")]
    JsExecution(#[from] JsError),
}

// ============================================================================
// 中断控制器
// ============================================================================

/// 中断控制器，用于控制 JS 执行的中断
#[derive(Debug)]
pub struct InterruptController {
    /// 中断标志
    interrupted: AtomicBool,
    /// 开始执行时间 (Unix 时间戳毫秒)
    start_time_ms: AtomicU64,
    /// 超时时间 (毫秒)
    timeout_ms: AtomicU64,
    /// 操作计数器 (用于减少时间检查频率)
    op_counter: AtomicU64,
}

impl InterruptController {
    /// 创建新的中断控制器
    pub fn new() -> Self {
        Self {
            interrupted: AtomicBool::new(false),
            start_time_ms: AtomicU64::new(0),
            timeout_ms: AtomicU64::new(DEFAULT_EXECUTION_TIMEOUT.as_millis() as u64),
            op_counter: AtomicU64::new(0),
        }
    }

    /// 设置超时时间
    pub fn set_timeout(&self, timeout: Duration) {
        self.timeout_ms.store(timeout.as_millis() as u64, Ordering::SeqCst);
    }

    /// 开始计时
    pub fn start(&self) {
        // 使用 unwrap_or 防止系统时间异常时 panic
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        self.start_time_ms.store(now, Ordering::SeqCst);
        self.interrupted.store(false, Ordering::SeqCst);
        self.op_counter.store(0, Ordering::SeqCst);
    }

    /// 手动触发中断
    pub fn interrupt(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
    }

    /// 检查是否应该中断 (由 interrupt_handler 调用)
    /// 返回 true 表示应该中断执行
    pub fn should_interrupt(&self) -> bool {
        // 首先检查是否被手动中断
        if self.interrupted.load(Ordering::SeqCst) {
            return true;
        }

        // 每 N 次操作检查一次超时，减少性能开销
        let count = self.op_counter.fetch_add(1, Ordering::Relaxed);
        if count % INTERRUPT_CHECK_INTERVAL != 0 {
            return false;
        }

        // 检查超时
        let start = self.start_time_ms.load(Ordering::SeqCst);
        if start == 0 {
            return false;
        }

        // 使用 unwrap_or 防止系统时间异常时 panic
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;

        let timeout = self.timeout_ms.load(Ordering::SeqCst);
        // 使用 saturating_sub 防止 underflow（时钟回退情况）
        if now.saturating_sub(start) > timeout {
            self.interrupted.store(true, Ordering::SeqCst);
            return true;
        }

        false
    }

    /// 重置控制器
    pub fn reset(&self) {
        self.interrupted.store(false, Ordering::SeqCst);
        self.start_time_ms.store(0, Ordering::SeqCst);
        self.op_counter.store(0, Ordering::SeqCst);
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 沙盒运行时
// ============================================================================

/// 沙盒运行时配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 内存限制 (字节)
    pub memory_limit: usize,
    /// 栈大小限制 (字节)
    pub stack_size: usize,
    /// 执行超时
    pub execution_timeout: Duration,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: DEFAULT_MEMORY_LIMIT,
            stack_size: DEFAULT_STACK_SIZE,
            execution_timeout: DEFAULT_EXECUTION_TIMEOUT,
        }
    }
}

/// 沙盒运行时
/// 封装 QuickJS AsyncRuntime，提供资源限制和超时控制
pub struct SandboxRuntime {
    /// QuickJS 异步运行时
    runtime: AsyncRuntime,
    /// 中断控制器
    interrupt_controller: Arc<InterruptController>,
    /// 配置
    config: SandboxConfig,
}

impl SandboxRuntime {
    /// 创建新的沙盒运行时
    pub async fn new(config: SandboxConfig) -> Result<Self, RuntimeError> {
        // 创建中断控制器
        let interrupt_controller = Arc::new(InterruptController::new());
        interrupt_controller.set_timeout(config.execution_timeout);

        // 创建 QuickJS 运行时
        let runtime = AsyncRuntime::new()
            .map_err(|e| RuntimeError::RuntimeCreation(e.to_string()))?;

        // 配置内存限制
        runtime.set_memory_limit(config.memory_limit).await;

        // 配置栈大小限制
        runtime.set_max_stack_size(config.stack_size).await;

        // 配置 interrupt_handler
        let controller = interrupt_controller.clone();
        runtime
            .set_interrupt_handler(Some(Box::new(move || controller.should_interrupt())))
            .await;

        log::info!(
            "沙盒运行时已创建: 内存限制={}MB, 栈大小={}KB, 超时={}s",
            config.memory_limit / 1024 / 1024,
            config.stack_size / 1024,
            config.execution_timeout.as_secs()
        );

        Ok(Self {
            runtime,
            interrupt_controller,
            config,
        })
    }

    /// 使用默认配置创建运行时
    pub async fn new_default() -> Result<Self, RuntimeError> {
        Self::new(SandboxConfig::default()).await
    }

    /// 创建新的执行上下文（不安全，仅用于测试）
    ///
    /// ⚠️ 警告：此方法不会初始化沙盒安全层，请使用 `create_sandboxed_context` 代替
    #[deprecated(note = "请使用 create_sandboxed_context 以确保安全隔离")]
    pub async fn create_context(&self) -> Result<AsyncContext, RuntimeError> {
        AsyncContext::full(&self.runtime)
            .await
            .map_err(|e| RuntimeError::ContextCreation(e.to_string()))
    }

    /// 创建沙盒化的执行上下文（推荐使用）
    ///
    /// 此方法会自动初始化沙盒安全层：
    /// - 注入安全 API（console、encoding、error）
    /// - 移除危险全局对象（eval、Function）
    pub async fn create_sandboxed_context(&self) -> Result<AsyncContext, RuntimeError> {
        let ctx = AsyncContext::full(&self.runtime)
            .await
            .map_err(|e| RuntimeError::ContextCreation(e.to_string()))?;

        // 强制初始化沙盒安全层
        SandboxApiInitializer::init_basic(&ctx)
            .await
            .map_err(|e| RuntimeError::ContextCreation(format!("沙盒初始化失败: {}", e)))?;

        log::info!("沙盒上下文已创建并初始化安全层");
        Ok(ctx)
    }

    /// 创建带 fetch 和 timer API 的沙盒上下文
    ///
    /// 此方法在基础沙盒上下文基础上，额外注入：
    /// - fetch API（受 SSRF 防护）
    /// - timer API（setTimeout/setInterval）
    ///
    /// # 参数
    /// - `permissions`: 允许的权限列表（如 ["fetch", "timer"]）
    pub async fn create_sandboxed_context_with_permissions(
        &self,
        permissions: &[String],
        request_manager: Option<Arc<RequestManager>>,
        timer_registry: Option<Arc<TimerRegistry>>,
    ) -> Result<AsyncContext, RuntimeError> {
        let ctx = AsyncContext::full(&self.runtime)
            .await
            .map_err(|e| RuntimeError::ContextCreation(e.to_string()))?;

        // 根据权限决定注入哪些 API
        let has_fetch = permissions.iter().any(|p| p == "fetch" || p == "network");
        let has_timer = permissions.iter().any(|p| p == "timer" || p == "setTimeout");

        if has_fetch {
            if let Some(rm) = request_manager {
                SandboxApiInitializer::init_with_fetch(&ctx, rm)
                    .await
                    .map_err(|e| RuntimeError::ContextCreation(format!("沙盒初始化失败: {}", e)))?;
            } else {
                // 没有 RequestManager，只初始化基础 API
                SandboxApiInitializer::init_basic(&ctx)
                    .await
                    .map_err(|e| RuntimeError::ContextCreation(format!("沙盒初始化失败: {}", e)))?;
            }
        } else {
            // 只初始化基础 API
            SandboxApiInitializer::init_basic(&ctx)
                .await
                .map_err(|e| RuntimeError::ContextCreation(format!("沙盒初始化失败: {}", e)))?;
        }

        // 注入 timer API（如果有权限）
        if has_timer {
            if let Some(tr) = timer_registry {
                ctx.with(|ctx| TimerApi::inject(&ctx, tr))
                    .await
                    .map_err(|e| RuntimeError::ContextCreation(format!("Timer API 注入失败: {}", e)))?;
            }
        }

        log::info!(
            "沙盒上下文已创建，权限: {:?}, fetch={}, timer={}",
            permissions, has_fetch, has_timer
        );
        Ok(ctx)
    }

    /// 获取中断控制器
    pub fn interrupt_controller(&self) -> Arc<InterruptController> {
        self.interrupt_controller.clone()
    }

    /// 开始执行计时
    pub fn start_execution(&self) {
        self.interrupt_controller.start();
    }

    /// 手动中断执行
    pub fn interrupt(&self) {
        self.interrupt_controller.interrupt();
    }

    /// 重置运行时状态
    pub fn reset(&self) {
        self.interrupt_controller.reset();
    }

    /// 获取配置
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// 获取运行时内存使用情况
    pub fn memory_usage(&self) -> usize {
        // rquickjs 0.6 不直接提供内存使用查询
        // 需要通过 Runtime::memory_usage() 方法，但在 async 版本中需要特殊处理
        0 // TODO: 实现实际的内存使用查询
    }

    /// 带资源限制的执行入口（推荐使用）
    /// 自动管理超时计时的启停，确保安全限制生效
    ///
    /// # 示例
    /// ```ignore
    /// let result = runtime.run_with_limits(&ctx, |ctx| {
    ///     ctx.eval::<i32, _>("1 + 2")
    /// }).await?;
    /// ```
    pub async fn run_with_limits<F, R>(&self, context: &AsyncContext, f: F) -> Result<R, RuntimeError>
    where
        F: for<'js> FnOnce(rquickjs::Ctx<'js>) -> rquickjs::Result<R> + Send,
        R: Send,
    {
        // 1. 启动 Watchdog
        let mut watchdog = Watchdog::new(self.interrupt_controller.clone());
        watchdog.start(self.config.execution_timeout);

        // 2. 开始计时
        self.start_execution();

        // 3. 执行代码
        let result = context.with(f).await;

        // 4. 停止 Watchdog
        watchdog.stop();

        // 5. 先缓存中断状态，再重置（修复超时检测失效问题）
        let was_interrupted = self.interrupt_controller.interrupted.load(Ordering::SeqCst);

        // 6. 重置状态
        self.reset();

        // 7. 检查是否是超时中断（使用缓存的状态）
        if was_interrupted {
            return Err(RuntimeError::ExecutionTimeout(self.config.execution_timeout));
        }

        result.map_err(RuntimeError::JsExecution)
    }

    /// 带自定义超时的执行入口
    pub async fn run_with_timeout<F, R>(
        &self,
        context: &AsyncContext,
        timeout: Duration,
        f: F,
    ) -> Result<R, RuntimeError>
    where
        F: for<'js> FnOnce(rquickjs::Ctx<'js>) -> rquickjs::Result<R> + Send,
        R: Send,
    {
        // 1. 启动 Watchdog
        let mut watchdog = Watchdog::new(self.interrupt_controller.clone());
        watchdog.start(timeout);

        // 临时设置超时
        let original_timeout = self.interrupt_controller.timeout_ms.load(Ordering::SeqCst);
        self.interrupt_controller.set_timeout(timeout);

        // 2. 开始计时
        self.start_execution();

        // 3. 执行代码
        let result = context.with(f).await;

        // 4. 停止 Watchdog
        watchdog.stop();

        // 5. 先缓存中断状态，再重置（修复超时检测失效问题）
        let was_interrupted = self.interrupt_controller.interrupted.load(Ordering::SeqCst);

        // 6. 重置状态
        self.reset();

        // 恢复原始超时设置
        self.interrupt_controller
            .timeout_ms
            .store(original_timeout, Ordering::SeqCst);

        // 7. 检查是否是超时中断（使用缓存的状态）
        if was_interrupted {
            return Err(RuntimeError::ExecutionTimeout(timeout));
        }

        result.map_err(RuntimeError::JsExecution)
    }
}

// ============================================================================
// Watchdog 任务
// ============================================================================

/// Watchdog 任务，独立于 JS 执行的超时检测
///
/// # 功能
/// - 在后台监控执行时间
/// - 超时时设置 interrupt 标志，让 interrupt_handler 在下次回调时中断执行
///
/// # 限制
/// - **无法中断阻塞型宿主函数**：如果 JS 代码调用了阻塞的 Rust 函数，
///   Watchdog 只能设置标志，必须等该函数返回后 interrupt_handler 才会生效
/// - 对于纯 JS CPU 循环，interrupt_handler 可以有效中断
/// - 建议：所有可能阻塞的宿主 API 应内置超时机制
pub struct Watchdog {
    /// 中断控制器
    controller: Arc<InterruptController>,
    /// 停止信号发送端
    stop_tx: Option<oneshot::Sender<()>>,
}

impl Watchdog {
    /// 创建新的 Watchdog
    pub fn new(controller: Arc<InterruptController>) -> Self {
        Self {
            controller,
            stop_tx: None,
        }
    }

    /// 启动 Watchdog 任务
    /// 返回一个可以用于停止 Watchdog 的句柄
    pub fn start(&mut self, timeout: Duration) {
        let (stop_tx, stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);

        let controller = self.controller.clone();

        tokio::spawn(async move {
            tokio::select! {
                biased;  // 优先检查 stop 信号，避免同时 ready 时随机选到 sleep 分支
                _ = stop_rx => {
                    // 正常停止
                    log::trace!("Watchdog 正常停止");
                }
                _ = tokio::time::sleep(timeout) => {
                    // 超时，触发中断
                    log::warn!("Watchdog 检测到超时 ({:?})，触发中断", timeout);
                    controller.interrupt();
                }
            }
        });

        log::trace!("Watchdog 已启动，超时时间: {:?}", timeout);
    }

    /// 停止 Watchdog 任务
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// 执行器
// ============================================================================

/// 带超时保护的 JS 执行器
pub struct Executor {
    runtime: Arc<SandboxRuntime>,
}

impl Executor {
    /// 创建新的执行器
    pub fn new(runtime: Arc<SandboxRuntime>) -> Self {
        Self { runtime }
    }

    /// 执行 JS 代码，带超时保护
    pub async fn execute_with_timeout<F, T>(
        &self,
        context: &AsyncContext,
        timeout: Duration,
        f: F,
    ) -> Result<T, RuntimeError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        // 启动 Watchdog
        let mut watchdog = Watchdog::new(self.runtime.interrupt_controller());
        watchdog.start(timeout);

        // 开始计时
        self.runtime.start_execution();

        // 执行代码
        let result = tokio::time::timeout(timeout, async {
            // 实际执行逻辑需要在 context.with 中进行
            // 这里仅作为框架示例
            Ok(f())
        })
        .await;

        // 停止 Watchdog
        watchdog.stop();

        // 重置状态
        self.runtime.reset();

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RuntimeError::ExecutionTimeout(timeout)),
        }
    }
}

// ============================================================================
// 插件执行器（安全入口）
// ============================================================================

/// 安全的插件执行器
///
/// 这是执行插件代码的唯一推荐入口，确保：
/// 1. 使用安全的沙盒上下文（自动初始化）
/// 2. 根据权限注入 API
/// 3. 应用资源限制和超时保护
pub struct PluginExecutor {
    runtime: Arc<SandboxRuntime>,
    request_manager: Option<Arc<RequestManager>>,
    timer_registry: Option<Arc<TimerRegistry>>,
}

impl PluginExecutor {
    /// 创建新的插件执行器
    pub fn new(runtime: Arc<SandboxRuntime>) -> Self {
        Self {
            runtime,
            request_manager: None,
            timer_registry: None,
        }
    }

    /// 设置 RequestManager（用于 fetch API）
    pub fn with_request_manager(mut self, rm: Arc<RequestManager>) -> Self {
        self.request_manager = Some(rm);
        self
    }

    /// 设置 TimerRegistry（用于 timer API）
    pub fn with_timer_registry(mut self, tr: Arc<TimerRegistry>) -> Self {
        self.timer_registry = Some(tr);
        self
    }

    /// 执行插件代码（安全入口）
    ///
    /// 自动创建沙盒上下文并根据权限注入 API
    /// 使用 Watchdog 和中断控制器确保超时保护生效
    ///
    /// # 安全保证
    /// 1. 使用安全的沙盒上下文（eval/Function 禁用）
    /// 2. 根据权限动态注入 API
    /// 3. 启用超时保护（Watchdog + interrupt_handler）
    /// 4. 内存限制生效
    ///
    /// # 参数
    /// - `code`: 要执行的 JS 代码
    /// - `permissions`: 插件权限列表
    ///
    /// # 返回
    /// - `Ok(Value)`: 执行结果
    /// - `Err(RuntimeError)`: 执行错误（超时、内存超限等）
    pub async fn execute_plugin(
        &self,
        code: &str,
        permissions: &[String],
    ) -> Result<serde_json::Value, RuntimeError> {
        // 1. 创建安全的沙盒上下文（根据权限注入 API）
        let ctx = self
            .runtime
            .create_sandboxed_context_with_permissions(
                permissions,
                self.request_manager.clone(),
                self.timer_registry.clone(),
            )
            .await?;

        // 2. 使用 run_with_limits 执行代码（启用超时保护）
        let code_owned = code.to_string();
        let json_str: String = self
            .runtime
            .run_with_limits(&ctx, move |js_ctx| -> rquickjs::Result<String> {
                // 执行插件代码
                let result: rquickjs::Value = js_ctx.eval(code_owned.as_bytes().to_vec())?;

                // 序列化结果为 JSON 字符串
                match js_ctx.json_stringify(result)? {
                    Some(s) => Ok(s.to_string()?),
                    None => Ok("null".to_string()),
                }
            })
            .await?;

        // 3. 解析 JSON 字符串
        serde_json::from_str(&json_str)
            .map_err(|e| RuntimeError::RuntimeCreation(format!("JSON parse error: {}", e)))
    }

    /// 执行插件文件（安全入口）
    ///
    /// 读取并执行插件入口文件
    /// 使用 O_NOFOLLOW 标志防止 TOCTOU 攻击
    ///
    /// # 安全保证
    /// 1. 使用 O_NOFOLLOW 标志打开文件，拒绝符号链接
    /// 2. 消除 entry_path() 检查与实际读取之间的 TOCTOU 窗口
    ///
    /// # 参数
    /// - `entry_path`: 插件入口文件路径（已通过安全校验）
    /// - `permissions`: 插件权限列表
    ///
    /// 参考：https://wiki.sei.cmu.edu/confluence/display/c/POS35-C
    pub async fn execute_plugin_file(
        &self,
        entry_path: &std::path::Path,
        permissions: &[String],
    ) -> Result<serde_json::Value, RuntimeError> {
        // 使用安全的文件读取方式（O_NOFOLLOW 防止 TOCTOU）
        let code = read_file_nofollow(entry_path).await?;

        // 执行代码
        self.execute_plugin(&code, permissions).await
    }
}

/// 安全读取文件（使用 O_NOFOLLOW 防止符号链接跟踪）
///
/// 这消除了路径检查与文件读取之间的 TOCTOU 窗口：
/// 攻击者无法在检查后将文件替换为符号链接
///
/// # 平台支持
/// - Unix: 使用 O_NOFOLLOW 标志
/// - Windows: 检查文件属性中的 reparse point
#[cfg(unix)]
async fn read_file_nofollow(path: &std::path::Path) -> Result<String, RuntimeError> {
    use std::os::unix::fs::OpenOptionsExt;
    use tokio::io::AsyncReadExt;

    // 在阻塞线程中打开文件（O_NOFOLLOW 需要同步操作）
    let path_owned = path.to_owned();
    let file = tokio::task::spawn_blocking(move || {
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path_owned)
    })
    .await
    .map_err(|e| RuntimeError::RuntimeCreation(format!("任务执行失败: {}", e)))?
    .map_err(|e| {
        if e.raw_os_error() == Some(libc::ELOOP) {
            RuntimeError::RuntimeCreation(format!(
                "拒绝打开符号链接文件（安全策略）: {}",
                path.display()
            ))
        } else {
            RuntimeError::RuntimeCreation(format!("读取插件文件失败: {}", e))
        }
    })?;

    // 转换为 tokio::fs::File 并读取内容
    let mut tokio_file = tokio::fs::File::from_std(file);
    let mut content = String::new();
    tokio_file
        .read_to_string(&mut content)
        .await
        .map_err(|e| RuntimeError::RuntimeCreation(format!("读取文件内容失败: {}", e)))?;

    Ok(content)
}

#[cfg(not(unix))]
async fn read_file_nofollow(path: &std::path::Path) -> Result<String, RuntimeError> {
    // Windows: 检查是否为 reparse point（符号链接/junction）
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|e| RuntimeError::RuntimeCreation(format!("获取文件元数据失败: {}", e)))?;

    if metadata.file_type().is_symlink() {
        return Err(RuntimeError::RuntimeCreation(format!(
            "拒绝打开符号链接文件（安全策略）: {}",
            path.display()
        )));
    }

    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| RuntimeError::RuntimeCreation(format!("读取插件文件失败: {}", e)))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_runtime_creation() {
        let runtime = SandboxRuntime::new_default().await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_sandbox_config() {
        let config = SandboxConfig {
            memory_limit: 8 * 1024 * 1024,
            stack_size: 256 * 1024,
            execution_timeout: Duration::from_secs(10),
        };

        let runtime = SandboxRuntime::new(config.clone()).await.unwrap();
        assert_eq!(runtime.config().memory_limit, 8 * 1024 * 1024);
        assert_eq!(runtime.config().stack_size, 256 * 1024);
    }

    #[tokio::test]
    async fn test_context_creation() {
        let runtime = SandboxRuntime::new_default().await.unwrap();
        let context = runtime.create_context().await;
        assert!(context.is_ok());
    }

    #[test]
    fn test_interrupt_controller() {
        let controller = InterruptController::new();

        // 初始状态不应中断
        assert!(!controller.should_interrupt());

        // 手动中断
        controller.interrupt();
        assert!(controller.should_interrupt());

        // 重置
        controller.reset();
        assert!(!controller.should_interrupt());
    }

    #[test]
    fn test_interrupt_controller_timeout() {
        let controller = InterruptController::new();
        controller.set_timeout(Duration::from_millis(10));
        controller.start();

        // 初始不应超时
        assert!(!controller.should_interrupt());

        // 等待超时
        std::thread::sleep(Duration::from_millis(20));

        // 需要触发足够多的操作才能检测到超时
        for _ in 0..INTERRUPT_CHECK_INTERVAL + 1 {
            controller.op_counter.fetch_add(1, Ordering::Relaxed);
        }

        // 重新检查应该检测到超时
        // 注意：由于 op_counter 已经超过检查间隔，下次调用会检查时间
        let controller2 = InterruptController::new();
        controller2.set_timeout(Duration::from_millis(1));
        controller2.start();
        std::thread::sleep(Duration::from_millis(5));

        // 模拟足够多的操作
        for _ in 0..INTERRUPT_CHECK_INTERVAL {
            let _ = controller2.should_interrupt();
        }
        // 这次调用应该检测到超时
        assert!(controller2.should_interrupt());
    }
}
