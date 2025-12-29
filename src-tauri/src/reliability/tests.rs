// Phase 3: 可靠性层集成测试
//
// 测试各组件的协同工作

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};

use crate::plugin::types::PluginErrorType;

use super::*;

// ============================================================================
// 集成测试: 调度器 + 限流器
// ============================================================================

#[tokio::test]
async fn test_scheduler_with_rate_limiter() {
    let rate_limiter = Arc::new(RateLimiter::with_default_config());
    let scheduler: TaskScheduler<i32> = TaskScheduler::with_default_config();

    let rate_limiter_clone = rate_limiter.clone();

    // 提交多个需要限流的任务
    let handles: Vec<_> = stream::iter(0..5)
        .then(|i| {
            let rl = rate_limiter_clone.clone();
            let sched = &scheduler;
            async move {
                sched.submit(async move {
                    rl.until_ready("test-plugin").await;
                    i
                }).await
            }
        })
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    // 等待所有任务完成
    let results: Vec<_> = futures::stream::iter(handles)
        .then(|h| h.await_result())
        .collect()
        .await;

    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.is_ok()));
}

// ============================================================================
// 集成测试: 缓存 + 重试
// ============================================================================

#[tokio::test]
async fn test_cache_with_retry() {
    let cache = Arc::new(CacheLayer::with_default_config());

    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    let key = CacheKey::new("test-plugin", "getData", &serde_json::json!({}));

    // 模拟一个会失败几次然后成功的操作
    let result = cache
        .get_or_compute(&key, false, || {
            let count = call_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err("temporary error".to_string())
                } else {
                    Ok(serde_json::json!({"success": true}))
                }
            }
        })
        .await;

    // 由于缓存层本身不做重试，这里应该失败
    assert!(result.is_err());
}

// ============================================================================
// 集成测试: 完整流程
// ============================================================================

struct TestOperationError {
    error_type: PluginErrorType,
    message: String,
}

impl retry::RetryableError for TestOperationError {
    fn is_retryable(&self) -> bool {
        self.error_type.is_retryable()
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }
}

#[tokio::test]
async fn test_full_reliability_pipeline() {
    // 创建所有组件
    let rate_limiter = Arc::new(RateLimiter::with_default_config());
    let cache = Arc::new(CacheLayer::with_default_config());
    let retry_executor = RetryExecutor::new(RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(10),
        ..Default::default()
    })
    .unwrap();

    let call_count = Arc::new(AtomicU32::new(0));
    let cache_key = CacheKey::new("test-plugin", "fetchData", &serde_json::json!({}));

    // 模拟完整的请求流程
    let rate_limiter_clone = rate_limiter.clone();
    let cache_clone = cache.clone();
    let call_count_clone = call_count.clone();
    let cache_key_clone = cache_key.clone();

    let result = retry_executor
        .execute(|| {
            let rl = rate_limiter_clone.clone();
            let c = cache_clone.clone();
            let count = call_count_clone.clone();
            let key = cache_key_clone.clone();

            async move {
                // 1. 限流检查
                rl.until_ready("test-plugin").await;

                // 2. 缓存检查
                if let Some(cached) = c.get(&key).await {
                    return Ok(cached);
                }

                // 3. 执行实际操作（模拟网络请求）
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(TestOperationError {
                        error_type: PluginErrorType::NetworkError,
                        message: "connection refused".to_string(),
                    })
                } else {
                    let value = serde_json::json!({"data": "success"});
                    // 4. 缓存结果
                    c.set(&key, value.clone()).await;
                    Ok(value)
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!({"data": "success"}));

    // 验证调用次数（2 次失败 + 1 次成功）
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    // 验证缓存已设置
    let cached = cache.get(&cache_key).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), serde_json::json!({"data": "success"}));

    // 验证统计
    let retry_stats = retry_executor.stats().snapshot();
    assert_eq!(retry_stats.total_retries, 2);
    assert_eq!(retry_stats.successful_retries, 1);
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[tokio::test]
async fn test_scheduler_queue_full() {
    // 测试配置：最大并发 1，队列大小 2
    // 当 1 个任务在执行 + 2 个任务在队列中时，第 4 个任务应该被拒绝
    let config = SchedulerConfig {
        max_concurrent: 1,
        max_queue_size: 2,
        ..Default::default()
    };
    let scheduler: TaskScheduler<()> = TaskScheduler::new(config);

    // 快速提交多个长时间运行的任务
    let mut success_count = 0;
    let mut fail_count = 0;

    for _ in 0..10 {
        let result = scheduler
            .submit(async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .await;

        match result {
            Ok(_) => success_count += 1,
            Err(scheduler::SchedulerError::QueueFull(_)) => fail_count += 1,
            _ => {}
        }
    }

    // 应该有一些成功（队列容量 + 并发数）和一些失败
    assert!(success_count > 0, "至少应该有一些任务成功提交");
    assert!(fail_count > 0, "当队列满时应该有任务被拒绝");
}

#[tokio::test]
async fn test_rate_limiter_plugin_isolation() {
    let limiter = RateLimiter::new(RateLimitConfig {
        global_rate_per_second: 100,
        global_burst: 50,
        plugin_rate_per_second: 2,
        plugin_burst: 1,
        ..Default::default()
    });

    // 插件 A 被限流
    assert!(limiter.check("plugin-a").await.is_ok());
    assert!(limiter.check("plugin-a").await.is_err());

    // 插件 B 不受影响
    assert!(limiter.check("plugin-b").await.is_ok());
    assert!(limiter.check("plugin-b").await.is_err());

    // 各自的统计独立
    let stats_a = limiter.plugin_stats("plugin-a").await.unwrap();
    let stats_b = limiter.plugin_stats("plugin-b").await.unwrap();

    assert_eq!(stats_a.1, 1);  // throttled
    assert_eq!(stats_b.1, 1);  // throttled
}

#[tokio::test]
async fn test_cache_different_params() {
    let cache = CacheLayer::with_default_config();

    // 相同插件和方法，不同参数
    let key1 = CacheKey::new("plugin", "method", &serde_json::json!({"id": 1}));
    let key2 = CacheKey::new("plugin", "method", &serde_json::json!({"id": 2}));

    cache.set(&key1, serde_json::json!({"value": 1})).await;
    cache.set(&key2, serde_json::json!({"value": 2})).await;

    // 应该是独立的缓存条目
    let v1 = cache.get(&key1).await.unwrap();
    let v2 = cache.get(&key2).await.unwrap();

    assert_eq!(v1, serde_json::json!({"value": 1}));
    assert_eq!(v2, serde_json::json!({"value": 2}));
}
