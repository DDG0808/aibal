// Phase 6.3: 告警机制
// 实现连续失败、高延迟、低成功率告警

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// 告警类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertType {
    /// 连续失败告警 (3 次失败触发)
    ConsecutiveFailures,
    /// 高延迟告警 (>5000ms)
    HighLatency,
    /// 低成功率告警 (<80%)
    LowSuccessRate,
}

impl AlertType {
    /// 获取告警类型名称
    pub fn name(&self) -> &'static str {
        match self {
            AlertType::ConsecutiveFailures => "连续失败",
            AlertType::HighLatency => "高延迟",
            AlertType::LowSuccessRate => "低成功率",
        }
    }
}

/// 告警级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    /// 警告
    Warning,
    /// 严重
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Warning => "警告",
            AlertSeverity::Critical => "严重",
        }
    }
}

/// 告警记录
#[derive(Debug, Clone)]
pub struct Alert {
    /// 告警类型
    pub alert_type: AlertType,
    /// 告警级别
    pub severity: AlertSeverity,
    /// 插件 ID
    pub plugin_id: String,
    /// 告警消息
    pub message: String,
    /// 触发时间
    pub timestamp: Instant,
    /// 额外数据
    pub data: Option<AlertData>,
}

/// 告警额外数据
#[derive(Debug, Clone)]
pub enum AlertData {
    /// 连续失败次数
    ConsecutiveFailures(u32),
    /// 高延迟值 (毫秒)
    HighLatency(f64),
    /// 低成功率值 (0-1)
    LowSuccessRate(f64),
}

/// 告警阈值配置
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// 连续失败触发阈值
    pub consecutive_failures: u32,
    /// 高延迟阈值 (毫秒)
    pub high_latency_ms: f64,
    /// 低成功率阈值 (0-1)
    pub low_success_rate: f64,
    /// 告警冷却时间 (秒)
    pub cooldown_seconds: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            consecutive_failures: 3,
            high_latency_ms: 5000.0,
            low_success_rate: 0.8,
            cooldown_seconds: 300, // 5 分钟冷却
        }
    }
}

/// 告警冷却键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CooldownKey {
    plugin_id: String,
    alert_type: AlertType,
}

/// 告警管理器
///
/// 管理插件告警，支持：
/// - 连续失败告警
/// - 高延迟告警
/// - 低成功率告警
/// - 告警去重和冷却（防止告警风暴）
pub struct AlertManager {
    /// 告警阈值配置
    thresholds: AlertThresholds,
    /// 告警历史
    history: RwLock<VecDeque<Alert>>,
    /// 历史记录最大数量
    max_history: usize,
    /// 冷却映射 (plugin_id + alert_type -> 上次告警时间)
    cooldown_map: RwLock<HashMap<CooldownKey, Instant>>,
    /// 通知回调
    notification_handler: Option<Arc<dyn NotificationHandler + Send + Sync>>,
}

/// 通知处理器 trait
///
/// 用于解耦告警管理器和具体的通知实现（如 tauri-plugin-notification）
pub trait NotificationHandler: Send + Sync {
    /// 发送通知
    fn send_notification(&self, title: &str, body: &str, is_critical: bool);
}

impl AlertManager {
    /// 创建新的告警管理器
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            thresholds,
            history: RwLock::new(VecDeque::with_capacity(100)),
            max_history: 100,
            cooldown_map: RwLock::new(HashMap::new()),
            notification_handler: None,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(AlertThresholds::default())
    }

    /// 设置通知处理器
    pub fn set_notification_handler<H: NotificationHandler + 'static>(&mut self, handler: H) {
        self.notification_handler = Some(Arc::new(handler));
    }

    /// 检查连续失败告警
    ///
    /// 当连续失败次数达到阈值时触发告警
    pub async fn check_consecutive_failures(&self, plugin_id: &str, consecutive_failures: u32) {
        if consecutive_failures >= self.thresholds.consecutive_failures {
            let severity = if consecutive_failures >= self.thresholds.consecutive_failures * 2 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };

            self.trigger_alert(
                AlertType::ConsecutiveFailures,
                severity,
                plugin_id,
                format!(
                    "插件 {} 连续失败 {} 次",
                    plugin_id, consecutive_failures
                ),
                Some(AlertData::ConsecutiveFailures(consecutive_failures)),
            )
            .await;
        }
    }

    /// 检查高延迟告警
    ///
    /// 当延迟超过阈值时触发告警
    pub async fn check_high_latency(&self, plugin_id: &str, latency_ms: f64) {
        if latency_ms > self.thresholds.high_latency_ms {
            let severity = if latency_ms > self.thresholds.high_latency_ms * 2.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };

            self.trigger_alert(
                AlertType::HighLatency,
                severity,
                plugin_id,
                format!(
                    "插件 {} 响应延迟过高: {:.0}ms (阈值: {:.0}ms)",
                    plugin_id, latency_ms, self.thresholds.high_latency_ms
                ),
                Some(AlertData::HighLatency(latency_ms)),
            )
            .await;
        }
    }

    /// 检查低成功率告警
    ///
    /// 当成功率低于阈值时触发告警
    pub async fn check_low_success_rate(&self, plugin_id: &str, success_rate: f64) {
        if success_rate < self.thresholds.low_success_rate {
            let severity = if success_rate < self.thresholds.low_success_rate / 2.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };

            self.trigger_alert(
                AlertType::LowSuccessRate,
                severity,
                plugin_id,
                format!(
                    "插件 {} 成功率过低: {:.1}% (阈值: {:.1}%)",
                    plugin_id,
                    success_rate * 100.0,
                    self.thresholds.low_success_rate * 100.0
                ),
                Some(AlertData::LowSuccessRate(success_rate)),
            )
            .await;
        }
    }

    /// 触发告警
    ///
    /// P1 修复：原子化冷却检查与更新，避免并发竞态。
    async fn trigger_alert(
        &self,
        alert_type: AlertType,
        severity: AlertSeverity,
        plugin_id: &str,
        message: String,
        data: Option<AlertData>,
    ) {
        let key = CooldownKey {
            plugin_id: plugin_id.to_string(),
            alert_type,
        };

        // P1 修复：在同一个写锁临界区完成冷却检查与更新
        {
            let mut cooldown_map = self.cooldown_map.write().await;
            let cooldown = Duration::from_secs(self.thresholds.cooldown_seconds);

            if let Some(&last_time) = cooldown_map.get(&key) {
                // P2 修复：缓存 elapsed 避免多次调用导致下溢
                let elapsed = last_time.elapsed();
                if elapsed < cooldown {
                    // 使用 saturating_sub 避免下溢 panic
                    let remaining = cooldown.saturating_sub(elapsed);
                    log::debug!(
                        "告警冷却中: {} - {} (剩余 {:.0}s)",
                        plugin_id,
                        alert_type.name(),
                        remaining.as_secs_f64()
                    );
                    return;
                }
            }

            // 原子更新冷却时间
            cooldown_map.insert(key, Instant::now());
        }

        // 创建告警
        let alert = Alert {
            alert_type,
            severity,
            plugin_id: plugin_id.to_string(),
            message: message.clone(),
            timestamp: Instant::now(),
            data,
        };

        // 记录告警
        {
            let mut history = self.history.write().await;
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(alert.clone());
        }

        // 发送日志
        match severity {
            AlertSeverity::Warning => {
                log::warn!("[告警] {}", message);
            }
            AlertSeverity::Critical => {
                log::error!("[严重告警] {}", message);
            }
        }

        // 发送系统通知
        if let Some(ref handler) = self.notification_handler {
            let title = format!("[{}] 插件告警", severity.as_str());
            handler.send_notification(&title, &message, severity == AlertSeverity::Critical);
        }
    }

    /// 获取告警历史
    pub async fn get_history(&self) -> Vec<Alert> {
        self.history.read().await.iter().cloned().collect()
    }

    /// 获取指定插件的告警历史
    pub async fn get_plugin_history(&self, plugin_id: &str) -> Vec<Alert> {
        self.history
            .read()
            .await
            .iter()
            .filter(|a| a.plugin_id == plugin_id)
            .cloned()
            .collect()
    }

    /// 清除告警历史
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// 清除冷却（用于测试）
    pub async fn clear_cooldown(&self) {
        self.cooldown_map.write().await.clear();
    }

    /// 获取告警统计
    pub async fn get_stats(&self) -> AlertStats {
        let history = self.history.read().await;

        let mut by_type: HashMap<AlertType, usize> = HashMap::new();
        let mut by_severity: HashMap<AlertSeverity, usize> = HashMap::new();

        for alert in history.iter() {
            *by_type.entry(alert.alert_type).or_insert(0) += 1;
            *by_severity.entry(alert.severity).or_insert(0) += 1;
        }

        AlertStats {
            total: history.len(),
            consecutive_failures: by_type.get(&AlertType::ConsecutiveFailures).copied().unwrap_or(0),
            high_latency: by_type.get(&AlertType::HighLatency).copied().unwrap_or(0),
            low_success_rate: by_type.get(&AlertType::LowSuccessRate).copied().unwrap_or(0),
            warnings: by_severity.get(&AlertSeverity::Warning).copied().unwrap_or(0),
            criticals: by_severity.get(&AlertSeverity::Critical).copied().unwrap_or(0),
        }
    }
}

/// 告警统计
#[derive(Debug, Clone)]
pub struct AlertStats {
    /// 总告警数
    pub total: usize,
    /// 连续失败告警数
    pub consecutive_failures: usize,
    /// 高延迟告警数
    pub high_latency: usize,
    /// 低成功率告警数
    pub low_success_rate: usize,
    /// 警告级别数
    pub warnings: usize,
    /// 严重级别数
    pub criticals: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consecutive_failures_alert() {
        let manager = AlertManager::with_defaults();

        // 不应触发（低于阈值）
        manager.check_consecutive_failures("test-plugin", 2).await;
        assert_eq!(manager.get_history().await.len(), 0);

        // 应触发
        manager.check_consecutive_failures("test-plugin", 3).await;
        let history = manager.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].alert_type, AlertType::ConsecutiveFailures);
    }

    #[tokio::test]
    async fn test_high_latency_alert() {
        let manager = AlertManager::with_defaults();

        // 不应触发
        manager.check_high_latency("test-plugin", 4000.0).await;
        assert_eq!(manager.get_history().await.len(), 0);

        // 应触发
        manager.check_high_latency("test-plugin", 6000.0).await;
        let history = manager.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].alert_type, AlertType::HighLatency);
    }

    #[tokio::test]
    async fn test_low_success_rate_alert() {
        let manager = AlertManager::with_defaults();

        // 不应触发
        manager.check_low_success_rate("test-plugin", 0.85).await;
        assert_eq!(manager.get_history().await.len(), 0);

        // 应触发
        manager.check_low_success_rate("test-plugin", 0.75).await;
        let history = manager.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].alert_type, AlertType::LowSuccessRate);
    }

    #[tokio::test]
    async fn test_cooldown() {
        let thresholds = AlertThresholds {
            cooldown_seconds: 1, // 1 秒冷却用于测试
            ..Default::default()
        };
        let manager = AlertManager::new(thresholds);

        // 第一次触发
        manager.check_consecutive_failures("test-plugin", 5).await;
        assert_eq!(manager.get_history().await.len(), 1);

        // 冷却期内不触发
        manager.check_consecutive_failures("test-plugin", 5).await;
        assert_eq!(manager.get_history().await.len(), 1);

        // 等待冷却结束
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 冷却结束后触发
        manager.check_consecutive_failures("test-plugin", 5).await;
        assert_eq!(manager.get_history().await.len(), 2);
    }

    #[tokio::test]
    async fn test_severity_levels() {
        let manager = AlertManager::with_defaults();

        // 普通警告 (3次失败)
        manager.check_consecutive_failures("plugin1", 3).await;
        let history = manager.get_history().await;
        assert_eq!(history[0].severity, AlertSeverity::Warning);

        // 严重告警 (6次失败 = 阈值*2)
        manager.check_consecutive_failures("plugin2", 6).await;
        let history = manager.get_history().await;
        assert_eq!(history[1].severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_cooldown_concurrent_atomicity() {
        use std::sync::Arc;

        let thresholds = AlertThresholds {
            cooldown_seconds: 60, // 长冷却时间
            ..Default::default()
        };
        let manager = Arc::new(AlertManager::new(thresholds));

        // 并发触发多个告警
        let mut handles = vec![];
        for i in 0..10 {
            let m = manager.clone();
            let plugin_id = format!("test-plugin-{}", i % 2); // 只有 2 个不同的 plugin
            handles.push(tokio::spawn(async move {
                m.check_consecutive_failures(&plugin_id, 5).await;
            }));
        }

        // 等待所有任务完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 由于冷却机制，每个 plugin 只应该触发 1 次告警
        let history = manager.get_history().await;
        assert_eq!(history.len(), 2, "每个 plugin 应该只触发 1 次告警");
    }

    #[tokio::test]
    async fn test_cooldown_different_alert_types() {
        let thresholds = AlertThresholds {
            cooldown_seconds: 60,
            ..Default::default()
        };
        let manager = AlertManager::new(thresholds);

        // 同一个 plugin 的不同告警类型应该分别触发
        manager.check_consecutive_failures("test-plugin", 5).await;
        manager.check_high_latency("test-plugin", 10000.0).await;
        manager.check_low_success_rate("test-plugin", 0.5).await;

        let history = manager.get_history().await;
        assert_eq!(history.len(), 3, "不同告警类型应该分别触发");
    }
}
