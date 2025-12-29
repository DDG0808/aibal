// 事件总线模块
// Phase 4.1: 通信总线
//
// 实现任务:
// - 4.1.1 实现 context.emit(event, data) - 事件可发布
// - 4.1.2 实现 subscribedEvents 解析 - 声明式订阅生效
// - 4.1.3 实现 onEvent 回调分发 - 事件路由正确
// - 4.1.4 实现事件队列 - 异步处理不阻塞

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

// ============================================================================
// 事件类型定义
// ============================================================================

/// 事件前缀类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPrefix {
    /// 插件事件: plugin:{plugin_id}:{action}
    Plugin,
    /// 系统事件: system:{action}
    System,
    /// IPC 事件: ipc:{action}
    Ipc,
}

impl EventPrefix {
    /// 从事件名称解析前缀
    pub fn from_event_name(name: &str) -> Option<Self> {
        if name.starts_with("plugin:") {
            Some(EventPrefix::Plugin)
        } else if name.starts_with("system:") {
            Some(EventPrefix::System)
        } else if name.starts_with("ipc:") {
            Some(EventPrefix::Ipc)
        } else {
            None
        }
    }
}

/// 队列中的事件
#[derive(Debug, Clone)]
pub struct QueuedEvent {
    /// 完整事件名称 (如 plugin:claude-usage:data_updated)
    pub event_name: String,
    /// 事件数据
    pub data: serde_json::Value,
    /// 事件来源插件 ID (系统事件为 None)
    pub source_plugin: Option<String>,
    /// 事件时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl QueuedEvent {
    /// 创建插件事件
    pub fn plugin_event(plugin_id: &str, action: &str, data: serde_json::Value) -> Self {
        Self {
            event_name: format!("plugin:{}:{}", plugin_id, action),
            data,
            source_plugin: Some(plugin_id.to_string()),
            timestamp: chrono::Utc::now(),
        }
    }

    /// 创建系统事件
    pub fn system_event(action: &str, data: serde_json::Value) -> Self {
        Self {
            event_name: format!("system:{}", action),
            data,
            source_plugin: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// 解析事件的插件 ID (仅插件事件)
    pub fn parse_plugin_id(&self) -> Option<String> {
        if self.event_name.starts_with("plugin:") {
            let parts: Vec<&str> = self.event_name.splitn(3, ':').collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
        None
    }
}

/// 事件分发结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDispatchResult {
    /// 成功分发的订阅者数量
    pub success_count: usize,
    /// 失败的订阅者
    pub failures: Vec<EventDispatchFailure>,
}

/// 事件分发失败信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDispatchFailure {
    /// 订阅者插件 ID
    pub plugin_id: String,
    /// 错误信息
    pub error: String,
}

// ============================================================================
// 事件总线错误
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("事件队列已关闭")]
    QueueClosed,

    #[error("事件队列已满")]
    QueueFull,

    #[error("无效的事件名称: {0}")]
    InvalidEventName(String),

    #[error("事件处理超时: {0}")]
    HandlerTimeout(String),

    #[error("事件处理失败: {0}")]
    HandlerError(String),
}

// ============================================================================
// 事件处理器回调
// ============================================================================

/// 事件处理器类型
///
/// 接收事件名称、数据，返回处理结果
pub type EventHandler = Arc<
    dyn Fn(String, serde_json::Value) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), String>> + Send>,
    > + Send
        + Sync,
>;

// ============================================================================
// 事件总线
// ============================================================================

/// 事件总线配置
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// 事件队列大小
    pub queue_size: usize,
    /// 事件处理超时 (毫秒)
    pub handler_timeout_ms: u64,
    /// 最大并发处理数
    pub max_concurrent_handlers: usize,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            queue_size: 1000,
            handler_timeout_ms: 5000,
            max_concurrent_handlers: 10,
        }
    }
}

/// 事件总线
///
/// 实现插件间的发布/订阅通信机制
pub struct EventBus {
    /// 订阅映射: event_name -> Set<plugin_id>
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,
    /// 事件处理器映射: plugin_id -> handler
    handlers: RwLock<HashMap<String, EventHandler>>,
    /// 事件发送通道
    event_tx: mpsc::Sender<QueuedEvent>,
    /// 事件接收通道 (后台任务持有)
    event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<QueuedEvent>>>,
    /// 配置
    config: EventBusConfig,
    /// 统计信息
    stats: RwLock<EventBusStats>,
}

/// 事件总线统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventBusStats {
    /// 已发布事件总数
    pub events_published: u64,
    /// 已分发事件总数
    pub events_dispatched: u64,
    /// 分发失败次数
    pub dispatch_failures: u64,
    /// 当前订阅者总数
    pub total_subscriptions: usize,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new(config: EventBusConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.queue_size);

        Self {
            subscriptions: RwLock::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
            event_tx: tx,
            event_rx: Arc::new(tokio::sync::Mutex::new(rx)),
            config,
            stats: RwLock::new(EventBusStats::default()),
        }
    }

    /// 使用默认配置创建
    pub fn new_default() -> Self {
        Self::new(EventBusConfig::default())
    }

    // ========================================================================
    // 订阅管理
    // ========================================================================

    /// 注册插件的事件订阅 (4.1.2)
    ///
    /// 解析插件的 subscribedEvents 声明，建立订阅关系
    ///
    /// # 参数
    /// - `plugin_id`: 订阅者插件 ID
    /// - `events`: 订阅的事件列表 (完整事件名)
    pub async fn subscribe(&self, plugin_id: &str, events: &[String]) {
        let mut subs = self.subscriptions.write().await;

        for event_name in events {
            // 验证事件名称格式
            if !Self::is_valid_event_name(event_name) {
                log::warn!("插件 {} 订阅了无效的事件名称: {}", plugin_id, event_name);
                continue;
            }

            subs.entry(event_name.clone())
                .or_insert_with(HashSet::new)
                .insert(plugin_id.to_string());

            log::debug!("插件 {} 订阅事件: {}", plugin_id, event_name);
        }

        // 更新统计
        let total = subs.values().map(|s| s.len()).sum();
        self.stats.write().await.total_subscriptions = total;
    }

    /// 取消插件的所有订阅
    ///
    /// 在插件卸载时调用，清理订阅关系和事件处理器
    pub async fn unsubscribe_all(&self, plugin_id: &str) {
        let mut subs = self.subscriptions.write().await;

        for subscribers in subs.values_mut() {
            subscribers.remove(plugin_id);
        }

        // 清理空的订阅条目
        subs.retain(|_, subscribers| !subscribers.is_empty());

        // 移除处理器
        self.handlers.write().await.remove(plugin_id);

        // 更新统计
        let total = subs.values().map(|s| s.len()).sum();
        self.stats.write().await.total_subscriptions = total;

        log::debug!("已取消插件 {} 的所有事件订阅和处理器", plugin_id);
    }

    /// 只取消插件的订阅（保留事件处理器）
    ///
    /// 在插件重载时调用，只清理订阅关系，保留已注册的事件处理器
    /// 这样 reload 后不需要重新执行插件代码来注册 handler
    pub async fn unsubscribe_only(&self, plugin_id: &str) {
        let mut subs = self.subscriptions.write().await;

        for subscribers in subs.values_mut() {
            subscribers.remove(plugin_id);
        }

        // 清理空的订阅条目
        subs.retain(|_, subscribers| !subscribers.is_empty());

        // 注意：不移除处理器，保留已注册的 onEvent 回调

        // 更新统计
        let total = subs.values().map(|s| s.len()).sum();
        self.stats.write().await.total_subscriptions = total;

        log::debug!("已取消插件 {} 的事件订阅（保留处理器）", plugin_id);
    }

    /// 注册事件处理器
    ///
    /// 为插件注册 onEvent 回调函数
    pub async fn register_handler(&self, plugin_id: &str, handler: EventHandler) {
        self.handlers.write().await.insert(plugin_id.to_string(), handler);
        log::debug!("已注册插件 {} 的事件处理器", plugin_id);
    }

    /// 获取事件的所有订阅者
    pub async fn get_subscribers(&self, event_name: &str) -> Vec<String> {
        self.subscriptions
            .read()
            .await
            .get(event_name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    // ========================================================================
    // 事件发布
    // ========================================================================

    /// 发布插件事件 (4.1.1)
    ///
    /// 插件调用 context.emit() 时触发
    /// 自动添加 "plugin:{pluginId}:" 前缀
    ///
    /// # 参数
    /// - `plugin_id`: 发布者插件 ID
    /// - `action`: 事件动作 (如 "data_updated")
    /// - `data`: 事件数据
    pub async fn emit(
        &self,
        plugin_id: &str,
        action: &str,
        data: serde_json::Value,
    ) -> Result<(), EventBusError> {
        // 验证 action 格式 (snake_case)
        if !Self::is_valid_action(action) {
            return Err(EventBusError::InvalidEventName(format!(
                "无效的 action 格式: {}",
                action
            )));
        }

        let event = QueuedEvent::plugin_event(plugin_id, action, data);
        self.queue_event(event).await
    }

    /// 发布插件事件（同步版本，用于 JS 回调）
    ///
    /// 使用 try_send 而非 send，适用于无法使用 async 的场景
    /// 如 JS 函数回调中的 context.emit()
    ///
    /// # 参数
    /// - `plugin_id`: 发布者插件 ID
    /// - `action`: 事件动作 (如 "data_updated")
    /// - `data`: 事件数据
    pub fn emit_sync(
        &self,
        plugin_id: &str,
        action: &str,
        data: serde_json::Value,
    ) -> Result<(), EventBusError> {
        // 验证 action 格式 (snake_case)
        if !Self::is_valid_action(action) {
            return Err(EventBusError::InvalidEventName(format!(
                "无效的 action 格式: {}",
                action
            )));
        }

        let event = QueuedEvent::plugin_event(plugin_id, action, data);

        // 使用 try_send 同步发送
        self.event_tx
            .try_send(event)
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => EventBusError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => EventBusError::QueueClosed,
            })?;

        // 更新统计（使用 try_write 非阻塞，失败则跳过统计更新）
        // 统计是可容错的，不阻塞主流程
        if let Ok(mut stats) = self.stats.try_write() {
            stats.events_published += 1;
        }

        log::trace!("事件已入队 (sync): plugin:{}:{}", plugin_id, action);
        Ok(())
    }

    /// 发布系统事件
    ///
    /// 由应用核心发布的全局事件
    pub async fn emit_system(
        &self,
        action: &str,
        data: serde_json::Value,
    ) -> Result<(), EventBusError> {
        let event = QueuedEvent::system_event(action, data);
        self.queue_event(event).await
    }

    /// 将事件放入队列 (4.1.4)
    async fn queue_event(&self, event: QueuedEvent) -> Result<(), EventBusError> {
        // 更新统计
        self.stats.write().await.events_published += 1;

        // 发送到队列
        self.event_tx
            .send(event)
            .await
            .map_err(|_| EventBusError::QueueClosed)?;

        Ok(())
    }

    // ========================================================================
    // 事件分发
    // ========================================================================

    /// 分发单个事件到所有订阅者 (4.1.3)
    ///
    /// 遍历订阅者，调用其 onEvent 回调
    /// 注意：先复制 handler 列表，释放锁后再执行 await，避免持锁 await 导致死锁
    async fn dispatch_event(&self, event: &QueuedEvent) -> EventDispatchResult {
        let subscribers = self.get_subscribers(&event.event_name).await;

        if subscribers.is_empty() {
            log::trace!("事件 {} 无订阅者", event.event_name);
            return EventDispatchResult {
                success_count: 0,
                failures: vec![],
            };
        }

        log::debug!(
            "分发事件 {} 到 {} 个订阅者",
            event.event_name,
            subscribers.len()
        );

        // 先复制 handler 列表，避免持锁 await
        let handlers_to_call: Vec<(String, EventHandler)> = {
            let handlers = self.handlers.read().await;
            subscribers
                .into_iter()
                .filter(|sub| event.source_plugin.as_ref() != Some(sub)) // 跳过事件来源插件
                .filter_map(|sub| {
                    handlers.get(&sub).map(|h| (sub, h.clone()))
                })
                .collect()
        };
        // 锁已释放

        let timeout = std::time::Duration::from_millis(self.config.handler_timeout_ms);
        let mut success_count = 0;
        let mut failures = Vec::new();

        for (subscriber, handler) in handlers_to_call {
            let event_name = event.event_name.clone();
            let data = event.data.clone();

            // 带超时的事件处理（无持锁）
            let result = tokio::time::timeout(timeout, async move {
                handler(event_name, data).await
            })
            .await;

            match result {
                Ok(Ok(())) => {
                    success_count += 1;
                }
                Ok(Err(e)) => {
                    log::warn!("插件 {} 处理事件失败: {}", subscriber, e);
                    failures.push(EventDispatchFailure {
                        plugin_id: subscriber,
                        error: e,
                    });
                }
                Err(_) => {
                    log::warn!("插件 {} 处理事件超时", subscriber);
                    failures.push(EventDispatchFailure {
                        plugin_id: subscriber,
                        error: "处理超时".to_string(),
                    });
                }
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.events_dispatched += 1;
            stats.dispatch_failures += failures.len() as u64;
        }

        EventDispatchResult {
            success_count,
            failures,
        }
    }

    /// 启动事件处理循环
    ///
    /// 在后台运行，从队列中取出事件并分发
    pub async fn start_dispatcher(self: Arc<Self>) {
        log::info!("事件总线分发器已启动");

        let mut rx = self.event_rx.lock().await;

        while let Some(event) = rx.recv().await {
            let event_name = event.event_name.clone();
            let result = self.dispatch_event(&event).await;

            if !result.failures.is_empty() {
                log::debug!(
                    "事件 {} 分发: {} 成功, {} 失败",
                    event_name,
                    result.success_count,
                    result.failures.len()
                );
            }
        }

        log::info!("事件总线分发器已停止");
    }

    /// 在 Tokio 运行时中启动分发器
    pub fn spawn_dispatcher(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.start_dispatcher().await;
        })
    }

    // ========================================================================
    // 工具方法
    // ========================================================================

    /// 验证事件名称格式
    fn is_valid_event_name(name: &str) -> bool {
        // 必须有前缀
        let prefix = EventPrefix::from_event_name(name);
        if prefix.is_none() {
            return false;
        }

        // 插件事件必须是三段式
        if name.starts_with("plugin:") {
            let parts: Vec<&str> = name.split(':').collect();
            return parts.len() == 3 && !parts[1].is_empty() && !parts[2].is_empty();
        }

        // 系统/IPC 事件必须是两段式
        let parts: Vec<&str> = name.split(':').collect();
        parts.len() == 2 && !parts[1].is_empty()
    }

    /// 验证 action 格式 (snake_case)
    fn is_valid_action(action: &str) -> bool {
        if action.is_empty() {
            return false;
        }

        // 只允许小写字母、数字和下划线
        action.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> EventBusStats {
        self.stats.read().await.clone()
    }

    /// 获取配置
    pub fn config(&self) -> &EventBusConfig {
        &self.config
    }
}

// ============================================================================
// 预定义系统事件
// ============================================================================

/// 预定义的系统事件 action
pub mod system_events {
    /// 用户触发全部刷新
    pub const REFRESH_ALL: &str = "refresh_all";
    /// 应用启动完成
    pub const APP_READY: &str = "app_ready";
    /// 应用即将退出
    pub const APP_WILL_QUIT: &str = "app_will_quit";
    /// 应用配置变更
    pub const CONFIG_CHANGED: &str = "config_changed";
    /// 网络状态变化
    pub const NETWORK_CHANGED: &str = "network_changed";
    /// 插件配置变更
    pub const PLUGIN_CONFIG_CHANGED: &str = "plugin_config_changed";
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_name_validation() {
        // 有效的事件名称
        assert!(EventBus::is_valid_event_name("plugin:claude-usage:data_updated"));
        assert!(EventBus::is_valid_event_name("system:refresh_all"));
        assert!(EventBus::is_valid_event_name("ipc:plugin_installed"));

        // 无效的事件名称
        assert!(!EventBus::is_valid_event_name("invalid"));
        assert!(!EventBus::is_valid_event_name("plugin:"));
        assert!(!EventBus::is_valid_event_name("plugin:id:"));
        assert!(!EventBus::is_valid_event_name("plugin::action"));
        assert!(!EventBus::is_valid_event_name("unknown:action"));
    }

    #[test]
    fn test_action_validation() {
        // 有效的 action
        assert!(EventBus::is_valid_action("data_updated"));
        assert!(EventBus::is_valid_action("threshold_exceeded"));
        assert!(EventBus::is_valid_action("session_reset_123"));

        // 无效的 action
        assert!(!EventBus::is_valid_action(""));
        assert!(!EventBus::is_valid_action("dataUpdated")); // camelCase
        assert!(!EventBus::is_valid_action("data-updated")); // kebab-case
        assert!(!EventBus::is_valid_action("Data_Updated")); // 大写
    }

    #[test]
    fn test_queued_event_creation() {
        let event = QueuedEvent::plugin_event(
            "claude-usage",
            "data_updated",
            serde_json::json!({"percentage": 75}),
        );

        assert_eq!(event.event_name, "plugin:claude-usage:data_updated");
        assert_eq!(event.source_plugin, Some("claude-usage".to_string()));
        assert_eq!(event.parse_plugin_id(), Some("claude-usage".to_string()));
    }

    #[test]
    fn test_system_event_creation() {
        let event = QueuedEvent::system_event(
            "refresh_all",
            serde_json::json!({"force": true}),
        );

        assert_eq!(event.event_name, "system:refresh_all");
        assert_eq!(event.source_plugin, None);
        assert_eq!(event.parse_plugin_id(), None);
    }

    #[tokio::test]
    async fn test_subscription() {
        let bus = EventBus::new_default();

        bus.subscribe(
            "notifications",
            &[
                "plugin:claude-usage:threshold_exceeded".to_string(),
                "plugin:claude-status:status_changed".to_string(),
            ],
        )
        .await;

        let subscribers = bus.get_subscribers("plugin:claude-usage:threshold_exceeded").await;
        assert_eq!(subscribers, vec!["notifications"]);

        let stats = bus.get_stats().await;
        assert_eq!(stats.total_subscriptions, 2);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let bus = EventBus::new_default();

        bus.subscribe(
            "plugin-a",
            &["plugin:claude-usage:data_updated".to_string()],
        )
        .await;

        bus.subscribe(
            "plugin-b",
            &["plugin:claude-usage:data_updated".to_string()],
        )
        .await;

        bus.unsubscribe_all("plugin-a").await;

        let subscribers = bus.get_subscribers("plugin:claude-usage:data_updated").await;
        assert_eq!(subscribers, vec!["plugin-b"]);
    }

    #[tokio::test]
    async fn test_emit_event() {
        let bus = EventBus::new_default();

        let result = bus
            .emit("claude-usage", "data_updated", serde_json::json!({"percentage": 50}))
            .await;

        assert!(result.is_ok());

        let stats = bus.get_stats().await;
        assert_eq!(stats.events_published, 1);
    }

    #[tokio::test]
    async fn test_emit_invalid_action() {
        let bus = EventBus::new_default();

        // camelCase action 应该失败
        let result = bus
            .emit("claude-usage", "dataUpdated", serde_json::json!({}))
            .await;

        assert!(result.is_err());
    }
}
