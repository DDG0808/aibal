// Phase 3.4: 重试机制
// 实现指数退避重试策略
//
// 任务:
// - 3.4.1 实现指数退避算法 ✓
// - 3.4.2 定义可重试错误类型 ✓
// - 3.4.3 实现最大重试次数限制 ✓
// - 3.4.4 实现重试统计 ✓

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use thiserror::Error;

use crate::plugin::types::PluginErrorType;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum RetryError {
    #[error("达到最大重试次数 ({max_retries}): {last_error}")]
    MaxRetriesExceeded {
        max_retries: u32,
        last_error: String,
    },

    #[error("不可重试的错误: {0}")]
    NonRetryable(String),

    #[error("重试被取消")]
    Cancelled,
}

/// 可重试错误特征
pub trait RetryableError {
    /// 检查错误是否可重试
    fn is_retryable(&self) -> bool;

    /// 获取错误描述
    fn error_message(&self) -> String;
}

/// 通用可重试错误包装
#[derive(Debug, Clone)]
pub struct RetryableErrorWrapper {
    pub error_type: PluginErrorType,
    pub message: String,
}

impl RetryableError for RetryableErrorWrapper {
    fn is_retryable(&self) -> bool {
        self.error_type.is_retryable()
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }
}

// ============================================================================
// 配置
// ============================================================================

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数（默认 3）
    pub max_retries: u32,
    /// 初始延迟（默认 100ms）
    pub initial_delay: Duration,
    /// 最大延迟（默认 5s）
    pub max_delay: Duration,
    /// 退避乘数（默认 2.0）
    pub multiplier: f64,
    /// Jitter 比例（默认 0.2，即 ±20%）
    pub jitter_factor: f64,
    /// 是否启用 Jitter（默认 true）
    pub enable_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
            jitter_factor: 0.2,
            enable_jitter: true,
        }
    }
}

/// 重试配置验证错误
#[derive(Debug, Clone, Error)]
pub enum RetryConfigError {
    #[error("multiplier 必须 >= 1.0，当前值: {0}")]
    InvalidMultiplier(f64),

    #[error("jitter_factor 必须在 0.0..=1.0 范围内，当前值: {0}")]
    InvalidJitterFactor(f64),

    #[error("initial_delay 必须 > 0")]
    InvalidInitialDelay,

    #[error("max_delay 必须 >= initial_delay")]
    InvalidMaxDelay,
}

impl RetryConfig {
    /// 验证配置有效性
    ///
    /// 检查：
    /// - multiplier >= 1.0
    /// - jitter_factor 在 [0.0, 1.0] 范围内
    /// - initial_delay > 0
    /// - max_delay >= initial_delay
    pub fn validate(&self) -> Result<(), RetryConfigError> {
        if self.multiplier < 1.0 {
            return Err(RetryConfigError::InvalidMultiplier(self.multiplier));
        }

        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err(RetryConfigError::InvalidJitterFactor(self.jitter_factor));
        }

        if self.initial_delay.is_zero() {
            return Err(RetryConfigError::InvalidInitialDelay);
        }

        if self.max_delay < self.initial_delay {
            return Err(RetryConfigError::InvalidMaxDelay);
        }

        Ok(())
    }

    /// 创建无重试配置
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// 创建激进重试配置（更多次数，更短间隔）
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(2),
            multiplier: 1.5,
            ..Default::default()
        }
    }

    /// 创建保守重试配置（较少次数，更长间隔）
    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            multiplier: 3.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// 统计信息
// ============================================================================

/// 重试统计
#[derive(Debug, Default)]
pub struct RetryStats {
    /// 总重试次数
    pub total_retries: AtomicU64,
    /// 重试后成功次数
    pub successful_retries: AtomicU64,
    /// 重试后仍失败次数
    pub failed_after_retries: AtomicU64,
    /// 首次成功次数（无需重试）
    pub immediate_successes: AtomicU64,
    /// 不可重试错误次数
    pub non_retryable_errors: AtomicU64,
}

impl RetryStats {
    pub fn snapshot(&self) -> RetryStatsSnapshot {
        let total_retries = self.total_retries.load(Ordering::Relaxed);
        let successful = self.successful_retries.load(Ordering::Relaxed);
        let failed = self.failed_after_retries.load(Ordering::Relaxed);

        RetryStatsSnapshot {
            total_retries,
            successful_retries: successful,
            failed_after_retries: failed,
            immediate_successes: self.immediate_successes.load(Ordering::Relaxed),
            non_retryable_errors: self.non_retryable_errors.load(Ordering::Relaxed),
            retry_success_rate: if total_retries > 0 {
                successful as f64 / total_retries as f64
            } else {
                0.0
            },
        }
    }

    fn record_retry(&self) {
        self.total_retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_successful_retry(&self) {
        self.successful_retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_failed_after_retries(&self) {
        self.failed_after_retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_immediate_success(&self) {
        self.immediate_successes.fetch_add(1, Ordering::Relaxed);
    }

    fn record_non_retryable(&self) {
        self.non_retryable_errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// 统计快照
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetryStatsSnapshot {
    pub total_retries: u64,
    pub successful_retries: u64,
    pub failed_after_retries: u64,
    pub immediate_successes: u64,
    pub non_retryable_errors: u64,
    pub retry_success_rate: f64,
}

// ============================================================================
// 重试执行器
// ============================================================================

/// 重试执行器
pub struct RetryExecutor {
    config: RetryConfig,
    stats: Arc<RetryStats>,
}

impl RetryExecutor {
    /// 创建新的重试执行器
    ///
    /// 会自动验证配置有效性，无效配置返回错误
    pub fn new(config: RetryConfig) -> Result<Self, RetryConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            stats: Arc::new(RetryStats::default()),
        })
    }

    /// 使用默认配置创建（默认配置保证有效，不会失败）
    pub fn with_default_config() -> Self {
        // 默认配置已验证为有效，可以安全 unwrap
        Self::new(RetryConfig::default()).expect("默认配置应始终有效")
    }

    /// 执行带重试的操作
    ///
    /// # 参数
    /// - `operation`: 要执行的异步操作
    ///
    /// # 返回
    /// - `Ok(T)`: 操作成功
    /// - `Err(RetryError)`: 达到最大重试次数或遇到不可重试错误
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, RetryError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: RetryableError,
    {
        self.execute_with_context(operation, |_, _| {}).await
    }

    /// 执行带重试的操作（带上下文回调）
    ///
    /// # 参数
    /// - `operation`: 要执行的异步操作
    /// - `on_retry`: 每次重试时的回调，参数为 (重试次数, 延迟时间)
    pub async fn execute_with_context<F, Fut, T, E, C>(
        &self,
        operation: F,
        mut on_retry: C,
    ) -> Result<T, RetryError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: RetryableError,
        C: FnMut(u32, Duration),
    {
        let mut attempt = 0;
        let mut last_error = String::new();

        loop {
            match operation().await {
                Ok(result) => {
                    if attempt == 0 {
                        self.stats.record_immediate_success();
                    } else {
                        self.stats.record_successful_retry();
                        log::debug!(
                            "操作在第 {} 次重试后成功",
                            attempt
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = e.error_message();

                    // 检查是否可重试
                    if !e.is_retryable() {
                        self.stats.record_non_retryable();
                        log::debug!("遇到不可重试错误: {}", last_error);
                        return Err(RetryError::NonRetryable(last_error));
                    }

                    // 检查是否达到最大重试次数
                    if attempt >= self.config.max_retries {
                        self.stats.record_failed_after_retries();
                        log::warn!(
                            "达到最大重试次数 ({}): {}",
                            self.config.max_retries,
                            last_error
                        );
                        return Err(RetryError::MaxRetriesExceeded {
                            max_retries: self.config.max_retries,
                            last_error,
                        });
                    }

                    attempt += 1;
                    self.stats.record_retry();

                    // 计算延迟时间
                    let delay = self.calculate_delay(attempt);
                    log::debug!(
                        "第 {} 次重试，延迟 {:?}: {}",
                        attempt,
                        delay,
                        last_error
                    );

                    on_retry(attempt, delay);

                    // 等待延迟
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// 计算退避延迟时间
    ///
    /// 安全边界：
    /// - multiplier 被 clamp 到 [1.0, 100.0]
    /// - 防止数值溢出
    fn calculate_delay(&self, attempt: u32) -> Duration {
        // 防御性编程：限制 multiplier 范围，防止指数爆炸
        let safe_multiplier = self.config.multiplier.clamp(1.0, 100.0);

        // 指数退避: initial_delay * (multiplier ^ (attempt - 1))
        // 使用 saturating 操作防止溢出
        let base_delay_ms = self.config.initial_delay.as_millis() as f64
            * safe_multiplier.powi((attempt as i32 - 1).min(20)); // 限制指数最大值

        // 限制最大延迟
        let capped_delay_ms = base_delay_ms
            .min(self.config.max_delay.as_millis() as f64)
            .min(u64::MAX as f64); // 防止 u64 溢出

        // 添加 Jitter
        let final_delay_ms = if self.config.enable_jitter {
            self.add_jitter(capped_delay_ms)
        } else {
            capped_delay_ms
        };

        Duration::from_millis(final_delay_ms as u64)
    }

    /// 添加随机 Jitter
    ///
    /// 安全边界：
    /// - jitter_factor 被 clamp 到 [0.0, 1.0]
    /// - 结果被 clamp 到 [0.0, max_delay]
    fn add_jitter(&self, delay_ms: f64) -> f64 {
        let mut rng = rand::thread_rng();
        // 防御性编程：限制 jitter_factor 范围
        let safe_factor = self.config.jitter_factor.clamp(0.0, 1.0);
        let jitter_range = delay_ms * safe_factor;
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        // 确保结果在合理范围内
        (delay_ms + jitter)
            .max(0.0)
            .min(self.config.max_delay.as_millis() as f64)
    }

    /// 获取统计信息
    pub fn stats(&self) -> &RetryStats {
        &self.stats
    }

    /// 获取配置
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 使用默认配置执行带重试的操作
pub async fn retry<F, Fut, T, E>(operation: F) -> Result<T, RetryError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetryableError,
{
    RetryExecutor::with_default_config().execute(operation).await
}

/// 使用指定配置执行带重试的操作
///
/// 注意：如果配置无效，会返回带有配置错误的 RetryError
pub async fn retry_with_config<F, Fut, T, E>(
    config: RetryConfig,
    operation: F,
) -> Result<T, RetryError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetryableError,
{
    let executor = RetryExecutor::new(config)
        .map_err(|e| RetryError::NonRetryable(format!("配置错误: {}", e)))?;
    executor.execute(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    // 测试用的可重试错误
    #[derive(Debug)]
    struct TestError {
        retryable: bool,
        message: String,
    }

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }

        fn error_message(&self) -> String {
            self.message.clone()
        }
    }

    #[tokio::test]
    async fn test_immediate_success() {
        let executor = RetryExecutor::with_default_config();

        let result = executor
            .execute(|| async { Ok::<_, TestError>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let stats = executor.stats().snapshot();
        assert_eq!(stats.immediate_successes, 1);
        assert_eq!(stats.total_retries, 0);
    }

    #[tokio::test]
    async fn test_retry_then_success() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let executor = RetryExecutor::new(RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        })
        .unwrap();

        let result = executor
            .execute(|| {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 {
                        Err(TestError {
                            retryable: true,
                            message: "temporary error".to_string(),
                        })
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);

        let stats = executor.stats().snapshot();
        assert_eq!(stats.successful_retries, 1);
        assert_eq!(stats.total_retries, 2);
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let executor = RetryExecutor::new(RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        })
        .unwrap();

        let result = executor
            .execute(|| async {
                Err::<i32, _>(TestError {
                    retryable: true,
                    message: "always fails".to_string(),
                })
            })
            .await;

        assert!(matches!(
            result,
            Err(RetryError::MaxRetriesExceeded { max_retries: 2, .. })
        ));

        let stats = executor.stats().snapshot();
        assert_eq!(stats.failed_after_retries, 1);
        assert_eq!(stats.total_retries, 2);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let executor = RetryExecutor::with_default_config();

        let result = executor
            .execute(|| async {
                Err::<i32, _>(TestError {
                    retryable: false,
                    message: "auth error".to_string(),
                })
            })
            .await;

        assert!(matches!(result, Err(RetryError::NonRetryable(_))));

        let stats = executor.stats().snapshot();
        assert_eq!(stats.non_retryable_errors, 1);
        assert_eq!(stats.total_retries, 0);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let delays = Arc::new(std::sync::Mutex::new(Vec::new()));
        let delays_clone = delays.clone();

        let executor = RetryExecutor::new(RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            enable_jitter: false,  // 禁用 jitter 以便精确测试
            ..Default::default()
        })
        .unwrap();

        let _ = executor
            .execute_with_context(
                || async {
                    Err::<i32, _>(TestError {
                        retryable: true,
                        message: "error".to_string(),
                    })
                },
                |_, delay| {
                    delays_clone.lock().unwrap().push(delay);
                },
            )
            .await;

        let delays = delays.lock().unwrap();
        assert_eq!(delays.len(), 3);

        // 验证指数退避: 100ms, 200ms, 400ms
        assert_eq!(delays[0], Duration::from_millis(100));
        assert_eq!(delays[1], Duration::from_millis(200));
        assert_eq!(delays[2], Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_max_delay_cap() {
        let delays = Arc::new(std::sync::Mutex::new(Vec::new()));
        let delays_clone = delays.clone();

        let executor = RetryExecutor::new(RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(300),
            multiplier: 2.0,
            enable_jitter: false,
            ..Default::default()
        })
        .unwrap();

        let _ = executor
            .execute_with_context(
                || async {
                    Err::<i32, _>(TestError {
                        retryable: true,
                        message: "error".to_string(),
                    })
                },
                |_, delay| {
                    delays_clone.lock().unwrap().push(delay);
                },
            )
            .await;

        let delays = delays.lock().unwrap();

        // 后面的延迟应该被限制在 300ms
        for i in 2..delays.len() {
            assert!(delays[i] <= Duration::from_millis(300));
        }
    }

    #[tokio::test]
    async fn test_jitter() {
        let delays = Arc::new(std::sync::Mutex::new(Vec::new()));
        let delays_clone = delays.clone();

        let executor = RetryExecutor::new(RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_millis(100),
            enable_jitter: true,
            jitter_factor: 0.2,
            ..Default::default()
        })
        .unwrap();

        let _ = executor
            .execute_with_context(
                || async {
                    Err::<i32, _>(TestError {
                        retryable: true,
                        message: "error".to_string(),
                    })
                },
                |_, delay| {
                    delays_clone.lock().unwrap().push(delay);
                },
            )
            .await;

        let delays = delays.lock().unwrap();

        // 由于 jitter，同一个重试次数的延迟应该有变化
        // 第一次重试的延迟应该在 80-120ms 之间（100ms ± 20%）
        assert!(delays[0] >= Duration::from_millis(80));
        assert!(delays[0] <= Duration::from_millis(120));
    }

    #[tokio::test]
    async fn test_no_retry_config() {
        let executor = RetryExecutor::new(RetryConfig::no_retry()).unwrap();

        let result = executor
            .execute(|| async {
                Err::<i32, _>(TestError {
                    retryable: true,
                    message: "error".to_string(),
                })
            })
            .await;

        assert!(matches!(
            result,
            Err(RetryError::MaxRetriesExceeded { max_retries: 0, .. })
        ));
    }

    #[tokio::test]
    async fn test_retry_convenience_function() {
        let result = retry(|| async { Ok::<_, TestError>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_plugin_error_type_retryable() {
        // 验证 PluginErrorType 的 is_retryable 实现
        assert!(PluginErrorType::NetworkError.is_retryable());
        assert!(PluginErrorType::Timeout.is_retryable());
        assert!(PluginErrorType::RateLimit.is_retryable());
        assert!(PluginErrorType::ProviderError.is_retryable());

        assert!(!PluginErrorType::AuthError.is_retryable());
        assert!(!PluginErrorType::ParseError.is_retryable());
        assert!(!PluginErrorType::PermissionDenied.is_retryable());
    }

    #[test]
    fn test_config_validation_in_new() {
        // 有效配置应该成功
        let result = RetryExecutor::new(RetryConfig::default());
        assert!(result.is_ok());

        // 无效 multiplier 应该失败
        let result = RetryExecutor::new(RetryConfig {
            multiplier: 0.5, // < 1.0
            ..Default::default()
        });
        assert!(matches!(result, Err(RetryConfigError::InvalidMultiplier(_))));

        // 无效 jitter_factor 应该失败
        let result = RetryExecutor::new(RetryConfig {
            jitter_factor: 1.5, // > 1.0
            ..Default::default()
        });
        assert!(matches!(result, Err(RetryConfigError::InvalidJitterFactor(_))));

        // 零 initial_delay 应该失败
        let result = RetryExecutor::new(RetryConfig {
            initial_delay: Duration::ZERO,
            ..Default::default()
        });
        assert!(matches!(result, Err(RetryConfigError::InvalidInitialDelay)));

        // max_delay < initial_delay 应该失败
        let result = RetryExecutor::new(RetryConfig {
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(1),
            ..Default::default()
        });
        assert!(matches!(result, Err(RetryConfigError::InvalidMaxDelay)));
    }
}
