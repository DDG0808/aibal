// Phase 3.3: 缓存层
// 基于 moka 的异步缓存实现
//
// 任务:
// - 3.3.1 集成 moka::future::Cache ✓
// - 3.3.2 实现 TTL 过期策略 ✓
// - 3.3.3 实现 TTI 空闲过期 ✓
// - 3.3.4 实现强制刷新 bypass ✓
// - 3.3.5 实现缓存命中率统计 ✓

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use moka::future::Cache;
use thiserror::Error;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("缓存未命中: {key}")]
    Miss { key: String },

    #[error("缓存值序列化失败: {0}")]
    SerializationError(String),

    #[error("缓存值反序列化失败: {0}")]
    DeserializationError(String),
}

// ============================================================================
// 配置
// ============================================================================

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大条目数（默认 1000）
    pub max_capacity: u64,
    /// TTL: 条目存活时间（默认 5 分钟）
    pub time_to_live: Duration,
    /// TTI: 条目空闲时间（默认 2 分钟）
    pub time_to_idle: Duration,
    /// 是否启用统计（默认 true）
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            time_to_live: Duration::from_secs(300),  // 5 分钟
            time_to_idle: Duration::from_secs(120),  // 2 分钟
            enable_stats: true,
        }
    }
}

// ============================================================================
// 缓存键
// ============================================================================

/// 缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// 插件 ID
    pub plugin_id: String,
    /// 方法名
    pub method: String,
    /// 参数哈希
    pub params_hash: u64,
}

impl CacheKey {
    /// 创建新的缓存键
    pub fn new(plugin_id: &str, method: &str, params: &serde_json::Value) -> Self {
        let params_hash = Self::hash_params(params);
        Self {
            plugin_id: plugin_id.to_string(),
            method: method.to_string(),
            params_hash,
        }
    }

    /// 计算参数哈希
    fn hash_params(params: &serde_json::Value) -> u64 {
        let mut hasher = DefaultHasher::new();
        // 将 JSON 转换为字符串进行哈希
        let json_str = serde_json::to_string(params).unwrap_or_default();
        json_str.hash(&mut hasher);
        hasher.finish()
    }

    /// 转换为字符串表示
    pub fn to_string_key(&self) -> String {
        format!("{}:{}:{}", self.plugin_id, self.method, self.params_hash)
    }
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_key())
    }
}

// ============================================================================
// 统计信息
// ============================================================================

/// 缓存统计
#[derive(Debug, Default)]
pub struct CacheStats {
    /// 命中次数
    pub hits: AtomicU64,
    /// 未命中次数
    pub misses: AtomicU64,
    /// 插入次数
    pub inserts: AtomicU64,
    /// 删除/过期次数
    pub evictions: AtomicU64,
    /// 强制刷新次数
    pub force_refreshes: AtomicU64,
}

impl CacheStats {
    /// 获取快照
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        CacheStatsSnapshot {
            hits,
            misses,
            inserts: self.inserts.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            force_refreshes: self.force_refreshes.load(Ordering::Relaxed),
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_insert(&self) {
        self.inserts.fetch_add(1, Ordering::Relaxed);
    }

    fn record_force_refresh(&self) {
        self.force_refreshes.fetch_add(1, Ordering::Relaxed);
    }
}

/// 统计快照
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub evictions: u64,
    pub force_refreshes: u64,
    pub hit_rate: f64,
}

// ============================================================================
// 缓存层
// ============================================================================

/// 异步缓存层
///
/// 支持：
/// - TTL 和 TTI 过期策略
/// - 强制刷新绕过缓存
/// - 缓存命中率统计
/// - 按插件 ID 批量失效缓存
pub struct CacheLayer {
    cache: Cache<String, serde_json::Value>,
    stats: Arc<CacheStats>,
    config: CacheConfig,
    /// 插件 ID -> 缓存键集合 的反向索引，用于按插件批量失效
    plugin_keys: DashMap<String, HashSet<String>>,
}

impl CacheLayer {
    /// 创建新的缓存层
    pub fn new(config: CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.time_to_live)
            .time_to_idle(config.time_to_idle)
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::default()),
            config,
            plugin_keys: DashMap::new(),
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(CacheConfig::default())
    }

    /// 注册缓存键到插件索引
    fn register_key(&self, key: &CacheKey) {
        self.plugin_keys
            .entry(key.plugin_id.clone())
            .or_default()
            .insert(key.to_string_key());
    }

    /// 从插件索引中移除缓存键
    fn unregister_key(&self, key: &CacheKey) {
        if let Some(mut keys) = self.plugin_keys.get_mut(&key.plugin_id) {
            keys.remove(&key.to_string_key());
        }
    }

    /// 获取缓存值
    pub async fn get(&self, key: &CacheKey) -> Option<serde_json::Value> {
        let key_str = key.to_string_key();

        match self.cache.get(&key_str).await {
            Some(value) => {
                self.stats.record_hit();
                log::trace!("缓存命中: {}", key);
                Some(value)
            }
            None => {
                self.stats.record_miss();
                log::trace!("缓存未命中: {}", key);
                None
            }
        }
    }

    /// 获取或计算值
    ///
    /// 如果缓存中存在则返回缓存值，否则执行计算函数并缓存结果
    pub async fn get_or_compute<F, Fut>(
        &self,
        key: &CacheKey,
        force: bool,
        compute: F,
    ) -> Result<serde_json::Value, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<serde_json::Value, String>>,
    {
        let key_str = key.to_string_key();

        // 如果不强制刷新，先尝试获取缓存
        if !force {
            if let Some(value) = self.cache.get(&key_str).await {
                self.stats.record_hit();
                log::trace!("缓存命中: {}", key);
                return Ok(value);
            }
        } else {
            self.stats.record_force_refresh();
            log::trace!("强制刷新: {}", key);
        }

        self.stats.record_miss();

        // 计算新值
        let value = compute()
            .await
            .map_err(|e| CacheError::SerializationError(e))?;

        // 插入缓存并注册到索引
        self.cache.insert(key_str, value.clone()).await;
        self.register_key(key);
        self.stats.record_insert();
        log::trace!("缓存插入: {}", key);

        Ok(value)
    }

    /// 设置缓存值
    pub async fn set(&self, key: &CacheKey, value: serde_json::Value) {
        let key_str = key.to_string_key();
        self.cache.insert(key_str, value).await;
        self.register_key(key);
        self.stats.record_insert();
        log::trace!("缓存设置: {}", key);
    }

    /// 删除缓存值
    pub async fn invalidate(&self, key: &CacheKey) {
        let key_str = key.to_string_key();
        self.cache.invalidate(&key_str).await;
        self.unregister_key(key);
        log::trace!("缓存失效: {}", key);
    }

    /// 删除插件的所有缓存
    ///
    /// 使用反向索引批量删除该插件的所有缓存条目
    pub async fn invalidate_plugin(&self, plugin_id: &str) {
        // 从索引中获取该插件的所有键
        let keys_to_invalidate: Vec<String> = self
            .plugin_keys
            .remove(plugin_id)
            .map(|(_, keys)| keys.into_iter().collect())
            .unwrap_or_default();

        let count = keys_to_invalidate.len();

        // 批量失效缓存
        for key_str in keys_to_invalidate {
            self.cache.invalidate(&key_str).await;
        }

        // 触发后台清理任务
        self.cache.run_pending_tasks().await;

        log::debug!(
            "插件缓存已失效: plugin={}, 删除条目数={}",
            plugin_id,
            count
        );
    }

    /// 清空所有缓存
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.plugin_keys.clear();
        self.cache.run_pending_tasks().await;
        log::debug!("清空所有缓存");
    }

    /// 获取统计信息
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 获取缓存大小
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }

    /// 获取配置
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// 运行待处理任务（触发过期清理）
    pub async fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks().await;
    }
}

// ============================================================================
// 插件专用缓存包装器
// ============================================================================

/// 插件缓存包装器
///
/// 为特定插件提供便捷的缓存操作
pub struct PluginCache {
    cache: Arc<CacheLayer>,
    plugin_id: String,
}

impl PluginCache {
    /// 创建插件缓存包装器
    pub fn new(cache: Arc<CacheLayer>, plugin_id: String) -> Self {
        Self { cache, plugin_id }
    }

    /// 获取缓存值
    pub async fn get(&self, method: &str, params: &serde_json::Value) -> Option<serde_json::Value> {
        let key = CacheKey::new(&self.plugin_id, method, params);
        self.cache.get(&key).await
    }

    /// 获取或计算值
    pub async fn get_or_compute<F, Fut>(
        &self,
        method: &str,
        params: &serde_json::Value,
        force: bool,
        compute: F,
    ) -> Result<serde_json::Value, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<serde_json::Value, String>>,
    {
        let key = CacheKey::new(&self.plugin_id, method, params);
        self.cache.get_or_compute(&key, force, compute).await
    }

    /// 设置缓存值
    pub async fn set(&self, method: &str, params: &serde_json::Value, value: serde_json::Value) {
        let key = CacheKey::new(&self.plugin_id, method, params);
        self.cache.set(&key, value).await;
    }

    /// 删除缓存值
    pub async fn invalidate(&self, method: &str, params: &serde_json::Value) {
        let key = CacheKey::new(&self.plugin_id, method, params);
        self.cache.invalidate(&key).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_cache() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "getData", &json!({"id": 1}));
        let value = json!({"result": "success"});

        // 设置值
        cache.set(&key, value.clone()).await;

        // 获取值
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "getData", &json!({}));

        let cached = cache.get(&key).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_get_or_compute() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "compute", &json!({"x": 1}));
        let mut compute_count = 0;

        // 第一次应该计算
        let result = cache
            .get_or_compute(&key, false, || {
                compute_count += 1;
                async { Ok(json!({"computed": true})) }
            })
            .await
            .unwrap();

        assert_eq!(result, json!({"computed": true}));

        // 第二次应该使用缓存
        let result2 = cache
            .get_or_compute(&key, false, || async { Ok(json!({"computed": false})) })
            .await
            .unwrap();

        assert_eq!(result2, json!({"computed": true}));  // 仍然是第一次的值
    }

    #[tokio::test]
    async fn test_force_refresh() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "compute", &json!({}));

        // 设置初始值
        cache.set(&key, json!({"version": 1})).await;

        // 强制刷新
        let result = cache
            .get_or_compute(&key, true, || async { Ok(json!({"version": 2})) })
            .await
            .unwrap();

        assert_eq!(result, json!({"version": 2}));

        // 验证缓存已更新
        let cached = cache.get(&key).await.unwrap();
        assert_eq!(cached, json!({"version": 2}));
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "getData", &json!({}));
        cache.set(&key, json!({"data": "value"})).await;

        // 验证存在
        assert!(cache.get(&key).await.is_some());

        // 失效
        cache.invalidate(&key).await;

        // 验证不存在
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = CacheLayer::with_default_config();

        let key = CacheKey::new("test-plugin", "getData", &json!({}));

        // 未命中
        let _ = cache.get(&key).await;

        // 设置
        cache.set(&key, json!({})).await;

        // 命中
        let _ = cache.get(&key).await;

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.misses, 1);
        assert_eq!(snapshot.hits, 1);
        assert_eq!(snapshot.inserts, 1);
        assert_eq!(snapshot.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let config = CacheConfig {
            time_to_live: Duration::from_millis(50),
            time_to_idle: Duration::from_millis(25),
            ..Default::default()
        };
        let cache = CacheLayer::new(config);

        let key = CacheKey::new("test-plugin", "getData", &json!({}));
        cache.set(&key, json!({"data": "value"})).await;

        // 立即访问应该存在
        assert!(cache.get(&key).await.is_some());

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;
        cache.run_pending_tasks().await;

        // 应该已过期
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_plugin_cache() {
        let cache = Arc::new(CacheLayer::with_default_config());
        let plugin_cache = PluginCache::new(cache.clone(), "my-plugin".to_string());

        let params = json!({"id": 123});
        plugin_cache
            .set("getData", &params, json!({"result": "ok"}))
            .await;

        let cached = plugin_cache.get("getData", &params).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), json!({"result": "ok"}));
    }

    #[tokio::test]
    async fn test_cache_key_hash() {
        // 相同参数应该产生相同的键
        let key1 = CacheKey::new("plugin", "method", &json!({"a": 1, "b": 2}));
        let key2 = CacheKey::new("plugin", "method", &json!({"a": 1, "b": 2}));

        assert_eq!(key1, key2);

        // 不同参数应该产生不同的键
        let key3 = CacheKey::new("plugin", "method", &json!({"a": 1, "b": 3}));
        assert_ne!(key1, key3);
    }

    /// 关键测试：验证 invalidate_plugin 能正确清除指定插件的所有缓存
    #[tokio::test]
    async fn test_invalidate_plugin() {
        let cache = CacheLayer::with_default_config();

        // 为 plugin-a 设置多个缓存
        let key_a1 = CacheKey::new("plugin-a", "method1", &json!({}));
        let key_a2 = CacheKey::new("plugin-a", "method2", &json!({"x": 1}));
        let key_a3 = CacheKey::new("plugin-a", "method3", &json!({"y": 2}));

        cache.set(&key_a1, json!({"data": "a1"})).await;
        cache.set(&key_a2, json!({"data": "a2"})).await;
        cache.set(&key_a3, json!({"data": "a3"})).await;

        // 为 plugin-b 设置缓存
        let key_b1 = CacheKey::new("plugin-b", "method1", &json!({}));
        cache.set(&key_b1, json!({"data": "b1"})).await;

        // 验证所有缓存存在
        assert!(cache.get(&key_a1).await.is_some());
        assert!(cache.get(&key_a2).await.is_some());
        assert!(cache.get(&key_a3).await.is_some());
        assert!(cache.get(&key_b1).await.is_some());

        // 失效 plugin-a 的所有缓存
        cache.invalidate_plugin("plugin-a").await;

        // 验证 plugin-a 的缓存已被清除
        assert!(
            cache.get(&key_a1).await.is_none(),
            "plugin-a 的 key_a1 应该被清除"
        );
        assert!(
            cache.get(&key_a2).await.is_none(),
            "plugin-a 的 key_a2 应该被清除"
        );
        assert!(
            cache.get(&key_a3).await.is_none(),
            "plugin-a 的 key_a3 应该被清除"
        );

        // 验证 plugin-b 的缓存不受影响
        assert!(
            cache.get(&key_b1).await.is_some(),
            "plugin-b 的缓存不应被影响"
        );
    }

    /// 测试 invalidate_plugin 对不存在的插件不会报错
    #[tokio::test]
    async fn test_invalidate_nonexistent_plugin() {
        let cache = CacheLayer::with_default_config();

        // 对不存在的插件调用 invalidate_plugin 不应 panic
        cache.invalidate_plugin("nonexistent-plugin").await;

        // 正常工作应该继续
        let key = CacheKey::new("test", "method", &json!({}));
        cache.set(&key, json!({"ok": true})).await;
        assert!(cache.get(&key).await.is_some());
    }
}
