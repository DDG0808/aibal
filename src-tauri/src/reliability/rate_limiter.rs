// Phase 3.2: 限流器
// 基于令牌桶算法的限流实现
//
// 任务:
// - 3.2.1 集成 governor crate ✓
// - 3.2.2 实现全局限流 ✓
// - 3.2.3 实现插件级限流 ✓
// - 3.2.4 实现 until_ready_with_jitter ✓
// - 3.2.5 实现限流统计 ✓

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Jitter, Quota, RateLimiter as GovernorRateLimiter,
};
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("请求被限流: 插件 {plugin_id}")]
    RateLimited { plugin_id: String },

    #[error("全局限流触发")]
    GlobalRateLimited,

    #[error("限流器配置错误: {0}")]
    ConfigError(String),
}

// ============================================================================
// 配置
// ============================================================================

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 全局限流: 每秒最大请求数（默认 100）
    pub global_rate_per_second: u32,
    /// 全局限流: 突发容量（默认 50）
    pub global_burst: u32,
    /// 插件级限流: 每秒最大请求数（默认 20）
    pub plugin_rate_per_second: u32,
    /// 插件级限流: 突发容量（默认 10）
    pub plugin_burst: u32,
    /// Jitter 最小延迟（默认 5ms）
    pub jitter_min: Duration,
    /// Jitter 最大延迟（默认 50ms）
    pub jitter_max: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_rate_per_second: 100,
            global_burst: 50,
            plugin_rate_per_second: 20,
            plugin_burst: 10,
            jitter_min: Duration::from_millis(5),
            jitter_max: Duration::from_millis(50),
        }
    }
}

// ============================================================================
// 统计信息
// ============================================================================

/// 限流统计
#[derive(Debug, Default)]
pub struct RateLimiterStats {
    /// 总请求数
    pub total_requests: AtomicU64,
    /// 被限流的请求数
    pub throttled_requests: AtomicU64,
    /// 等待限流后通过的请求数
    pub waited_requests: AtomicU64,
    /// 立即通过的请求数
    pub immediate_requests: AtomicU64,
}

impl RateLimiterStats {
    pub fn snapshot(&self) -> RateLimiterStatsSnapshot {
        let total = self.total_requests.load(Ordering::Relaxed);
        let throttled = self.throttled_requests.load(Ordering::Relaxed);

        RateLimiterStatsSnapshot {
            total_requests: total,
            throttled_requests: throttled,
            waited_requests: self.waited_requests.load(Ordering::Relaxed),
            immediate_requests: self.immediate_requests.load(Ordering::Relaxed),
            throttle_rate: if total > 0 {
                throttled as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

/// 统计快照
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimiterStatsSnapshot {
    pub total_requests: u64,
    pub throttled_requests: u64,
    pub waited_requests: u64,
    pub immediate_requests: u64,
    pub throttle_rate: f64,
}

/// 插件级别统计
#[derive(Debug, Default)]
pub struct PluginRateLimiterStats {
    pub total_requests: AtomicU64,
    pub throttled_requests: AtomicU64,
}

// ============================================================================
// 限流器类型别名
// ============================================================================

type InnerRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

// ============================================================================
// 限流器
// ============================================================================

/// 多层限流器
///
/// 支持全局限流和插件级别限流，使用令牌桶算法
pub struct RateLimiter {
    config: RateLimitConfig,
    /// 全局限流器
    global: InnerRateLimiter,
    /// 插件级限流器
    plugin_limiters: RwLock<HashMap<String, Arc<PluginLimiter>>>,
    /// 全局统计
    stats: Arc<RateLimiterStats>,
    /// Jitter 配置
    jitter: Jitter,
}

struct PluginLimiter {
    limiter: InnerRateLimiter,
    stats: PluginRateLimiterStats,
}

impl RateLimiter {
    /// 创建新的限流器
    ///
    /// 注意：如果配置值为 0，会使用安全默认值（1）并记录警告日志。
    /// 这确保了服务不会因配置问题启动失败，同时保持行为可追溯。
    pub fn new(config: RateLimitConfig) -> Self {
        // 验证全局速率配置，无效时使用安全默认值
        let global_rate = match NonZeroU32::new(config.global_rate_per_second) {
            Some(r) => r,
            None => {
                log::warn!(
                    "全局限流配置无效: global_rate_per_second=0，使用默认值 1"
                );
                NonZeroU32::MIN
            }
        };

        let global_burst = match NonZeroU32::new(config.global_burst) {
            Some(b) => b,
            None => {
                log::warn!(
                    "全局限流配置无效: global_burst=0，使用默认值 1"
                );
                NonZeroU32::MIN
            }
        };

        let global_quota = Quota::per_second(global_rate).allow_burst(global_burst);
        let global = GovernorRateLimiter::direct(global_quota);
        let jitter = Jitter::new(config.jitter_min, config.jitter_max);

        log::debug!(
            "创建限流器: 全局速率={}/s, 突发={}, 插件速率={}/s, 插件突发={}",
            global_rate,
            global_burst,
            config.plugin_rate_per_second,
            config.plugin_burst
        );

        Self {
            config,
            global,
            plugin_limiters: RwLock::new(HashMap::new()),
            stats: Arc::new(RateLimiterStats::default()),
            jitter,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// 检查请求是否被允许（非阻塞）
    pub async fn check(&self, plugin_id: &str) -> Result<(), RateLimitError> {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        // 全局限流检查
        if self.global.check().is_err() {
            self.stats.throttled_requests.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::GlobalRateLimited);
        }

        // 插件级限流检查
        let plugin_limiter = self.get_or_create_plugin_limiter(plugin_id).await;
        if plugin_limiter.limiter.check().is_err() {
            self.stats.throttled_requests.fetch_add(1, Ordering::Relaxed);
            plugin_limiter
                .stats
                .throttled_requests
                .fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::RateLimited {
                plugin_id: plugin_id.to_string(),
            });
        }

        plugin_limiter
            .stats
            .total_requests
            .fetch_add(1, Ordering::Relaxed);
        self.stats.immediate_requests.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// 等待直到请求被允许
    pub async fn until_ready(&self, plugin_id: &str) {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        // 全局限流等待
        self.global.until_ready().await;

        // 插件级限流等待
        let plugin_limiter = self.get_or_create_plugin_limiter(plugin_id).await;
        plugin_limiter.limiter.until_ready().await;

        plugin_limiter
            .stats
            .total_requests
            .fetch_add(1, Ordering::Relaxed);
        self.stats.waited_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// 等待直到请求被允许（带 Jitter 避免雷暴效应）
    pub async fn until_ready_with_jitter(&self, plugin_id: &str) {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        // 全局限流等待（带 Jitter）
        self.global.until_ready_with_jitter(self.jitter).await;

        // 插件级限流等待（带 Jitter）
        let plugin_limiter = self.get_or_create_plugin_limiter(plugin_id).await;
        plugin_limiter
            .limiter
            .until_ready_with_jitter(self.jitter)
            .await;

        plugin_limiter
            .stats
            .total_requests
            .fetch_add(1, Ordering::Relaxed);
        self.stats.waited_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取或创建插件级限流器
    ///
    /// 注意：如果 plugin_rate_per_second 或 plugin_burst 为 0，
    /// 会使用配置错误时的安全默认值（速率 1/s，突发 1）并记录警告。
    async fn get_or_create_plugin_limiter(&self, plugin_id: &str) -> Arc<PluginLimiter> {
        // 先尝试读取
        {
            let limiters = self.plugin_limiters.read().await;
            if let Some(limiter) = limiters.get(plugin_id) {
                return limiter.clone();
            }
        }

        // 需要创建新的限流器
        let mut limiters = self.plugin_limiters.write().await;

        // 双重检查
        if let Some(limiter) = limiters.get(plugin_id) {
            return limiter.clone();
        }

        // 验证配置，记录警告但不 panic
        let rate = match NonZeroU32::new(self.config.plugin_rate_per_second) {
            Some(r) => r,
            None => {
                log::warn!(
                    "插件 {} 限流配置无效: plugin_rate_per_second=0，使用默认值 1",
                    plugin_id
                );
                NonZeroU32::MIN
            }
        };

        let burst = match NonZeroU32::new(self.config.plugin_burst) {
            Some(b) => b,
            None => {
                log::warn!(
                    "插件 {} 限流配置无效: plugin_burst=0，使用默认值 1",
                    plugin_id
                );
                NonZeroU32::MIN
            }
        };

        let quota = Quota::per_second(rate).allow_burst(burst);

        let plugin_limiter = Arc::new(PluginLimiter {
            limiter: GovernorRateLimiter::direct(quota),
            stats: PluginRateLimiterStats::default(),
        });

        limiters.insert(plugin_id.to_string(), plugin_limiter.clone());
        log::debug!("为插件 {} 创建限流器 (rate={}/s, burst={})", plugin_id, rate, burst);

        plugin_limiter
    }

    /// 获取全局统计
    pub fn stats(&self) -> &RateLimiterStats {
        &self.stats
    }

    /// 获取插件统计
    pub async fn plugin_stats(&self, plugin_id: &str) -> Option<(u64, u64)> {
        let limiters = self.plugin_limiters.read().await;
        limiters.get(plugin_id).map(|l| {
            (
                l.stats.total_requests.load(Ordering::Relaxed),
                l.stats.throttled_requests.load(Ordering::Relaxed),
            )
        })
    }

    /// 重置插件限流器（插件卸载时调用）
    pub async fn remove_plugin(&self, plugin_id: &str) {
        let mut limiters = self.plugin_limiters.write().await;
        limiters.remove(plugin_id);
        log::debug!("移除插件 {} 的限流器", plugin_id);
    }

    /// 清空所有插件限流器
    pub async fn clear_all(&self) {
        let mut limiters = self.plugin_limiters.write().await;
        limiters.clear();
        log::debug!("清空所有插件限流器");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 10,
            global_burst: 5,
            plugin_rate_per_second: 5,
            plugin_burst: 2,
            ..Default::default()
        });

        // 前几个请求应该立即通过
        for _ in 0..2 {
            assert!(limiter.check("test-plugin").await.is_ok());
        }

        // 超过突发容量后应该被限流
        let mut throttled = 0;
        for _ in 0..10 {
            if limiter.check("test-plugin").await.is_err() {
                throttled += 1;
            }
        }

        assert!(throttled > 0);
    }

    #[tokio::test]
    async fn test_until_ready() {
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 10,
            global_burst: 2,
            plugin_rate_per_second: 5,
            plugin_burst: 1,
            ..Default::default()
        });

        let start = Instant::now();

        // 执行多个请求
        for _ in 0..5 {
            limiter.until_ready("test-plugin").await;
        }

        let elapsed = start.elapsed();

        // 5 个请求，速率 5/s，应该花费约 800ms（扣除突发容量）
        assert!(elapsed >= Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_jitter() {
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 10,
            global_burst: 1,
            plugin_rate_per_second: 5,
            plugin_burst: 1,
            jitter_min: Duration::from_millis(10),
            jitter_max: Duration::from_millis(50),
        });

        let start = Instant::now();

        // 执行多个请求
        for _ in 0..3 {
            limiter.until_ready_with_jitter("test-plugin").await;
        }

        let elapsed = start.elapsed();

        // 应该有 jitter 延迟
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_stats() {
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 10,
            global_burst: 2,
            plugin_rate_per_second: 5,
            plugin_burst: 1,
            ..Default::default()
        });

        // 执行一些请求
        for _ in 0..5 {
            let _ = limiter.check("test-plugin").await;
        }

        let snapshot = limiter.stats().snapshot();
        assert_eq!(snapshot.total_requests, 5);
        assert!(snapshot.throttled_requests > 0);
    }

    #[tokio::test]
    async fn test_multiple_plugins() {
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 100,
            global_burst: 50,
            plugin_rate_per_second: 5,
            plugin_burst: 2,
            ..Default::default()
        });

        // 不同插件有独立的限流
        for _ in 0..2 {
            assert!(limiter.check("plugin-a").await.is_ok());
            assert!(limiter.check("plugin-b").await.is_ok());
        }

        let stats_a = limiter.plugin_stats("plugin-a").await;
        let stats_b = limiter.plugin_stats("plugin-b").await;

        assert!(stats_a.is_some());
        assert!(stats_b.is_some());
    }

    #[tokio::test]
    async fn test_remove_plugin() {
        let limiter = RateLimiter::with_default_config();

        // 创建插件限流器
        limiter.until_ready("test-plugin").await;

        // 验证存在
        assert!(limiter.plugin_stats("test-plugin").await.is_some());

        // 移除
        limiter.remove_plugin("test-plugin").await;

        // 验证不存在
        assert!(limiter.plugin_stats("test-plugin").await.is_none());
    }

    #[tokio::test]
    async fn test_zero_config_fallback() {
        // 零配置应该不会 panic，而是使用默认值 1
        let limiter = RateLimiter::new(RateLimitConfig {
            global_rate_per_second: 0,  // 无效，会 fallback 到 1
            global_burst: 0,            // 无效，会 fallback 到 1
            plugin_rate_per_second: 0,  // 无效，会 fallback 到 1
            plugin_burst: 0,            // 无效，会 fallback 到 1
            ..Default::default()
        });

        // 应该能正常工作（虽然速率很低）
        // 第一个请求应该立即通过（利用突发容量）
        assert!(limiter.check("test-plugin").await.is_ok());

        // 后续请求会被限流（因为速率只有 1/s）
        let result = limiter.check("test-plugin").await;
        assert!(result.is_err());
    }
}
