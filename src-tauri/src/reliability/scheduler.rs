// Phase 3.1: 并发调度器
// 实现多插件并发执行、任务取消和队列管理
//
// 任务:
// - 3.1.1 集成 futures::stream ✓
// - 3.1.2 实现 buffer_unordered 并发 ✓
// - 3.1.3 实现 max_concurrent 配置 ✓
// - 3.1.4 实现任务取消机制 ✓
// - 3.1.5 实现任务队列 ✓

use std::collections::VecDeque;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use futures::FutureExt;
use thiserror::Error;
use tokio::sync::{oneshot, Mutex, Notify, Semaphore};
use tokio_util::sync::CancellationToken;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Clone, Error)]
pub enum SchedulerError {
    #[error("任务已取消")]
    Cancelled,

    #[error("任务超时: {0:?}")]
    Timeout(Duration),

    #[error("调度器已关闭")]
    Shutdown,

    #[error("队列已满: 最大容量 {0}")]
    QueueFull(usize),

    #[error("任务执行失败: {0}")]
    TaskFailed(String),

    #[error("任务 panic: {0}")]
    TaskPanic(String),
}

// ============================================================================
// 配置
// ============================================================================

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 最大并发任务数（默认 10）
    pub max_concurrent: usize,
    /// 任务队列最大长度（默认 100）
    pub max_queue_size: usize,
    /// 默认任务超时时间
    pub default_timeout: Duration,
    /// 是否启用优先级队列
    pub enable_priority: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            max_queue_size: 100,
            default_timeout: Duration::from_secs(30),
            enable_priority: true,
        }
    }
}

// ============================================================================
// 任务优先级
// ============================================================================

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// 低优先级（后台任务）
    Low = 0,
    /// 正常优先级
    Normal = 1,
    /// 高优先级（用户交互）
    High = 2,
    /// 紧急优先级（告警相关）
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

// ============================================================================
// 任务句柄
// ============================================================================

/// 任务句柄，用于取消和获取结果
pub struct TaskHandle<T> {
    /// 取消令牌
    cancel_token: CancellationToken,
    /// 结果接收器
    result_rx: oneshot::Receiver<Result<T, SchedulerError>>,
    /// 任务 ID
    task_id: u64,
}

impl<T> TaskHandle<T> {
    /// 取消任务
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 获取任务 ID
    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// 等待任务完成并获取结果
    pub async fn await_result(self) -> Result<T, SchedulerError> {
        self.result_rx.await.unwrap_or(Err(SchedulerError::Cancelled))
    }
}

// ============================================================================
// 调度器统计
// ============================================================================

/// 调度器统计信息
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// 总提交任务数
    pub total_submitted: AtomicU64,
    /// 已完成任务数
    pub total_completed: AtomicU64,
    /// 已取消任务数
    pub total_cancelled: AtomicU64,
    /// 超时任务数
    pub total_timeout: AtomicU64,
    /// 失败任务数
    pub total_failed: AtomicU64,
    /// panic 任务数
    pub total_panicked: AtomicU64,
    /// 当前活跃任务数
    pub active_count: AtomicUsize,
    /// 当前队列长度
    pub queue_length: AtomicUsize,
}

impl SchedulerStats {
    pub fn snapshot(&self) -> SchedulerStatsSnapshot {
        SchedulerStatsSnapshot {
            total_submitted: self.total_submitted.load(Ordering::Relaxed),
            total_completed: self.total_completed.load(Ordering::Relaxed),
            total_cancelled: self.total_cancelled.load(Ordering::Relaxed),
            total_timeout: self.total_timeout.load(Ordering::Relaxed),
            total_failed: self.total_failed.load(Ordering::Relaxed),
            total_panicked: self.total_panicked.load(Ordering::Relaxed),
            active_count: self.active_count.load(Ordering::Relaxed),
            queue_length: self.queue_length.load(Ordering::Relaxed),
        }
    }
}

/// 统计快照（用于序列化）
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStatsSnapshot {
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_cancelled: u64,
    pub total_timeout: u64,
    pub total_failed: u64,
    pub total_panicked: u64,
    pub active_count: usize,
    pub queue_length: usize,
}

// ============================================================================
// 内部任务表示
// ============================================================================

type BoxedTask<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

struct QueuedTask<T: Send + 'static> {
    id: u64,
    priority: TaskPriority,
    future: BoxedTask<T>,
    result_tx: oneshot::Sender<Result<T, SchedulerError>>,
    cancel_token: CancellationToken,
    timeout: Duration,
    queued_at: Instant,
}

// ============================================================================
// 任务调度器
// ============================================================================

/// 并发任务调度器
///
/// 支持：
/// - 最大并发数限制
/// - 优先级队列
/// - 任务取消
/// - 超时控制
pub struct TaskScheduler<T: Send + 'static = ()> {
    config: SchedulerConfig,
    /// 并发信号量
    semaphore: Arc<Semaphore>,
    /// 任务队列
    queue: Arc<Mutex<VecDeque<QueuedTask<T>>>>,
    /// 任务 ID 生成器
    next_task_id: AtomicU64,
    /// 统计信息
    stats: Arc<SchedulerStats>,
    /// 关闭令牌
    shutdown_token: CancellationToken,
    /// 队列唤醒通知（用于 permit 释放后唤醒后台 worker）
    queue_notify: Arc<Notify>,
}

impl<T: Send + 'static> TaskScheduler<T> {
    /// 创建新的调度器
    pub fn new(config: SchedulerConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let stats = Arc::new(SchedulerStats::default());
        let shutdown_token = CancellationToken::new();
        let queue_notify = Arc::new(Notify::new());

        // 启动后台 worker 持续处理队列
        let queue_clone = queue.clone();
        let semaphore_clone = semaphore.clone();
        let stats_clone = stats.clone();
        let shutdown_clone = shutdown_token.clone();
        let notify_clone = queue_notify.clone();

        tokio::spawn(async move {
            Self::background_worker(
                queue_clone,
                semaphore_clone,
                stats_clone,
                shutdown_clone,
                notify_clone,
            )
            .await;
        });

        Self {
            config,
            semaphore,
            queue,
            next_task_id: AtomicU64::new(1),
            stats,
            shutdown_token,
            queue_notify,
        }
    }

    /// 使用默认配置创建调度器
    pub fn with_default_config() -> Self {
        Self::new(SchedulerConfig::default())
    }

    /// 提交任务
    pub async fn submit<F>(
        &self,
        future: F,
    ) -> Result<TaskHandle<T>, SchedulerError>
    where
        F: Future<Output = T> + Send + 'static,
    {
        self.submit_with_options(future, TaskPriority::Normal, self.config.default_timeout)
            .await
    }

    /// 提交带选项的任务
    pub async fn submit_with_options<F>(
        &self,
        future: F,
        priority: TaskPriority,
        timeout: Duration,
    ) -> Result<TaskHandle<T>, SchedulerError>
    where
        F: Future<Output = T> + Send + 'static,
    {
        if self.shutdown_token.is_cancelled() {
            return Err(SchedulerError::Shutdown);
        }

        let task_id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let cancel_token = CancellationToken::new();
        let (result_tx, result_rx) = oneshot::channel();

        let task = QueuedTask {
            id: task_id,
            priority,
            future: Box::pin(future),
            result_tx,
            cancel_token: cancel_token.clone(),
            timeout,
            queued_at: Instant::now(),
        };

        // 添加到队列（在持有锁的情况下检查容量，确保原子性）
        {
            let mut queue = self.queue.lock().await;

            // 在持有锁的情况下检查队列容量（原子操作）
            if queue.len() >= self.config.max_queue_size {
                return Err(SchedulerError::QueueFull(self.config.max_queue_size));
            }

            if self.config.enable_priority {
                // 按优先级插入（高优先级在前）
                let pos = queue
                    .iter()
                    .position(|t| t.priority < priority)
                    .unwrap_or(queue.len());
                queue.insert(pos, task);
            } else {
                queue.push_back(task);
            }
        }

        self.stats.total_submitted.fetch_add(1, Ordering::Relaxed);
        self.stats.queue_length.fetch_add(1, Ordering::Relaxed);

        // 通知后台 worker 有新任务
        self.queue_notify.notify_one();

        log::trace!("任务 {} 已提交 (优先级: {:?})", task_id, priority);

        Ok(TaskHandle {
            cancel_token,
            result_rx,
            task_id,
        })
    }

    /// 后台 worker: 持续处理队列中的任务
    ///
    /// 核心逻辑：
    /// 1. 等待有任务提交（通过 Notify）
    /// 2. 获取 permit（阻塞等待，确保不超过并发限制）
    /// 3. 从队列取任务执行
    /// 4. 任务完成后释放 permit，并通知继续处理
    async fn background_worker(
        queue: Arc<Mutex<VecDeque<QueuedTask<T>>>>,
        semaphore: Arc<Semaphore>,
        stats: Arc<SchedulerStats>,
        shutdown: CancellationToken,
        notify: Arc<Notify>,
    ) {
        loop {
            // 检查是否关闭
            if shutdown.is_cancelled() {
                log::debug!("调度器后台 worker 关闭");
                break;
            }

            // 检查队列是否有任务
            let has_task = {
                let queue = queue.lock().await;
                !queue.is_empty()
            };

            if !has_task {
                // 队列为空，等待新任务通知
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => {
                        log::debug!("调度器后台 worker 关闭");
                        break;
                    }
                    _ = notify.notified() => {
                        // 收到通知，继续循环检查队列
                        continue;
                    }
                }
            }

            // 获取 permit（阻塞等待，确保不超过并发限制）
            let permit = tokio::select! {
                biased;
                _ = shutdown.cancelled() => {
                    log::debug!("调度器后台 worker 关闭");
                    break;
                }
                permit = semaphore.clone().acquire_owned() => {
                    match permit {
                        Ok(p) => p,
                        Err(_) => {
                            // Semaphore 被关闭
                            log::debug!("调度器信号量关闭");
                            break;
                        }
                    }
                }
            };

            // 从队列中取出任务
            let task = {
                let mut queue = queue.lock().await;
                queue.pop_front()
            };

            let task = match task {
                Some(t) => t,
                None => {
                    // 队列被其他 worker 抢空，释放 permit 继续等待
                    drop(permit);
                    continue;
                }
            };

            stats.queue_length.fetch_sub(1, Ordering::Relaxed);
            stats.active_count.fetch_add(1, Ordering::Relaxed);

            // 在单独的任务中执行
            let stats_clone = stats.clone();
            let notify_clone = notify.clone();
            tokio::spawn(async move {
                let result = Self::execute_task(task).await;

                // 更新统计
                match &result {
                    Ok(_) => {
                        stats_clone.total_completed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(SchedulerError::Cancelled) => {
                        stats_clone.total_cancelled.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(SchedulerError::Timeout(_)) => {
                        stats_clone.total_timeout.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(SchedulerError::TaskPanic(_)) => {
                        stats_clone.total_panicked.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        stats_clone.total_failed.fetch_add(1, Ordering::Relaxed);
                    }
                }

                stats_clone.active_count.fetch_sub(1, Ordering::Relaxed);

                // 释放 permit 并通知后台 worker 继续处理队列
                drop(permit);
                notify_clone.notify_one();
            });
        }
    }

    /// 执行单个任务
    ///
    /// 使用 catch_unwind 捕获任务 panic，确保：
    /// 1. 统计数据正确更新
    /// 2. 调用方收到明确的 TaskPanic 错误而非 Cancelled
    async fn execute_task(task: QueuedTask<T>) -> Result<(), SchedulerError> {
        let QueuedTask {
            id,
            future,
            result_tx,
            cancel_token,
            timeout,
            queued_at,
            ..
        } = task;

        let wait_time = queued_at.elapsed();
        log::trace!("任务 {} 开始执行 (等待时间: {:?})", id, wait_time);

        // 使用 catch_unwind 捕获任务 panic
        let result = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                log::trace!("任务 {} 已取消", id);
                Err(SchedulerError::Cancelled)
            }
            result = tokio::time::timeout(timeout, AssertUnwindSafe(future).catch_unwind()) => {
                match result {
                    Ok(Ok(value)) => {
                        log::trace!("任务 {} 执行完成", id);
                        Ok(value)
                    }
                    Ok(Err(panic_info)) => {
                        // 任务 panic
                        let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "未知 panic".to_string()
                        };
                        log::error!("任务 {} panic: {}", id, panic_msg);
                        Err(SchedulerError::TaskPanic(panic_msg))
                    }
                    Err(_) => {
                        log::warn!("任务 {} 超时 ({:?})", id, timeout);
                        Err(SchedulerError::Timeout(timeout))
                    }
                }
            }
        };

        // 保存错误类型用于统计
        let error_type = match &result {
            Ok(_) => None,
            Err(e) => Some(e.clone()),
        };

        // 发送结果（忽略发送失败，可能接收端已关闭）
        let _ = result_tx.send(result);

        // 返回状态用于统计
        match error_type {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }

    /// 并发执行多个任务
    ///
    /// 返回每个任务的执行结果。如果任务提交失败（如队列满），
    /// 该位置的结果也会是 Err。
    pub async fn execute_many<I, F>(
        &self,
        tasks: I,
    ) -> Vec<Result<T, SchedulerError>>
    where
        I: IntoIterator<Item = F>,
        F: Future<Output = T> + Send + 'static,
    {
        // 收集所有提交结果（包括失败的）
        let submit_results: Vec<_> = stream::iter(tasks)
            .then(|task| async { self.submit(task).await })
            .collect()
            .await;

        // 等待所有成功提交的任务完成，保留提交失败的错误
        let mut results = Vec::with_capacity(submit_results.len());
        for submit_result in submit_results {
            match submit_result {
                Ok(handle) => {
                    results.push(handle.await_result().await);
                }
                Err(e) => {
                    // 提交失败的任务也作为结果返回
                    results.push(Err(e));
                }
            }
        }

        results
    }

    /// 并发执行多个任务（使用 buffer_unordered）
    pub async fn execute_unordered<I, F>(
        &self,
        tasks: I,
        max_concurrent: usize,
    ) -> Vec<Result<T, SchedulerError>>
    where
        I: IntoIterator<Item = F>,
        F: Future<Output = T> + Send + 'static,
    {
        stream::iter(tasks)
            .map(|task| async move {
                match self.submit(task).await {
                    Ok(handle) => handle.await_result().await,
                    Err(e) => Err(e),
                }
            })
            .buffer_unordered(max_concurrent)
            .collect()
            .await
    }

    /// 获取统计信息
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// 获取当前活跃任务数
    pub fn active_count(&self) -> usize {
        self.stats.active_count.load(Ordering::Relaxed)
    }

    /// 获取当前队列长度
    pub fn queue_length(&self) -> usize {
        self.stats.queue_length.load(Ordering::Relaxed)
    }

    /// 关闭调度器
    pub async fn shutdown(&self) {
        self.shutdown_token.cancel();

        // 取消所有排队中的任务
        let mut queue = self.queue.lock().await;
        for task in queue.drain(..) {
            task.cancel_token.cancel();
            let _ = task.result_tx.send(Err(SchedulerError::Shutdown));
        }
    }
}

impl<T: Send + 'static> Drop for TaskScheduler<T> {
    fn drop(&mut self) {
        self.shutdown_token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_execution() {
        let scheduler: TaskScheduler<i32> = TaskScheduler::with_default_config();

        let handle = scheduler.submit(async { 42 }).await.unwrap();
        let result = handle.await_result().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let scheduler: TaskScheduler<i32> = TaskScheduler::with_default_config();

        let handle = scheduler.submit(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            42
        }).await.unwrap();

        handle.cancel();
        let result = handle.await_result().await;

        assert!(matches!(result, Err(SchedulerError::Cancelled)));
    }

    #[tokio::test]
    async fn test_task_timeout() {
        let config = SchedulerConfig {
            default_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let scheduler: TaskScheduler<i32> = TaskScheduler::new(config);

        let handle = scheduler.submit(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            42
        }).await.unwrap();

        let result = handle.await_result().await;

        assert!(matches!(result, Err(SchedulerError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let config = SchedulerConfig {
            max_concurrent: 5,
            ..Default::default()
        };
        let scheduler: TaskScheduler<u64> = TaskScheduler::new(config);

        let start = Instant::now();
        let results = scheduler.execute_unordered(
            (0..10).map(|i| async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                i
            }),
            5,
        ).await;
        let elapsed = start.elapsed();

        // 10 个任务，每个 100ms，5 并发 -> 约 200ms
        assert!(elapsed < Duration::from_millis(400));
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let config = SchedulerConfig {
            max_concurrent: 1,  // 限制为 1 以便观察顺序
            enable_priority: true,
            ..Default::default()
        };
        let scheduler: TaskScheduler<i32> = TaskScheduler::new(config);

        // 提交低优先级任务
        let _low = scheduler.submit_with_options(
            async { 1 },
            TaskPriority::Low,
            Duration::from_secs(30),
        ).await.unwrap();

        // 提交高优先级任务（应该排在前面）
        let _high = scheduler.submit_with_options(
            async { 2 },
            TaskPriority::High,
            Duration::from_secs(30),
        ).await.unwrap();

        // 检查队列长度
        assert!(scheduler.queue_length() > 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let scheduler: TaskScheduler<i32> = TaskScheduler::with_default_config();

        let handle = scheduler.submit(async { 42 }).await.unwrap();
        let _ = handle.await_result().await;

        // 等待统计更新
        tokio::time::sleep(Duration::from_millis(10)).await;

        let snapshot = scheduler.stats().snapshot();
        assert_eq!(snapshot.total_submitted, 1);
        assert_eq!(snapshot.total_completed, 1);
    }

    /// 关键测试：突发提交 N > max_concurrent 且无后续 submit 必须全部完成
    ///
    /// 这个测试验证 P0-1 修复：permit 释放后能正确唤醒后台 worker 继续处理队列
    #[tokio::test]
    async fn test_burst_submit_all_complete() {
        let config = SchedulerConfig {
            max_concurrent: 3,  // 最多同时 3 个任务
            max_queue_size: 100,
            default_timeout: Duration::from_secs(10),
            ..Default::default()
        };
        let scheduler: TaskScheduler<i32> = TaskScheduler::new(config);

        // 一次性提交 10 个任务（远超 max_concurrent=3）
        let mut handles = Vec::new();
        for i in 0..10 {
            let handle = scheduler.submit(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                i
            }).await.unwrap();
            handles.push(handle);
        }

        // 不再提交任何任务，等待所有任务完成
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await_result().await;
            results.push(result);
        }

        // 验证所有 10 个任务都成功完成
        assert_eq!(results.len(), 10);
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "任务 {} 应该成功完成", i);
        }

        // 验证统计
        tokio::time::sleep(Duration::from_millis(50)).await;
        let snapshot = scheduler.stats().snapshot();
        assert_eq!(snapshot.total_submitted, 10);
        assert_eq!(snapshot.total_completed, 10);
        assert_eq!(snapshot.queue_length, 0);
    }
}
