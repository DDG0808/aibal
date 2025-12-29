// Phase 6.3.4: tauri-plugin-notification 集成
// 实现系统通知发送功能

use super::alert::NotificationHandler;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Tauri 通知处理器
///
/// 使用 tauri-plugin-notification 发送系统通知
pub struct TauriNotificationHandler {
    app_handle: Arc<AppHandle>,
}

impl TauriNotificationHandler {
    /// 创建新的 Tauri 通知处理器
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app_handle),
        }
    }
}

impl NotificationHandler for TauriNotificationHandler {
    /// 发送系统通知
    ///
    /// 使用 tauri-plugin-notification 发送系统级通知
    fn send_notification(&self, title: &str, body: &str, is_critical: bool) {
        let builder = self.app_handle.notification().builder();

        // 构建通知
        let notification = builder
            .title(title)
            .body(body);

        // 发送通知
        if let Err(e) = notification.show() {
            log::error!("发送系统通知失败: {}", e);
        } else {
            if is_critical {
                log::warn!("已发送严重告警通知: {}", title);
            } else {
                log::info!("已发送告警通知: {}", title);
            }
        }
    }
}

/// 创建已配置通知处理器的告警管理器
///
/// 方便快速创建带通知功能的告警管理器
pub fn create_alert_manager_with_notifications(
    app_handle: AppHandle,
) -> super::AlertManager {
    let mut manager = super::AlertManager::with_defaults();
    manager.set_notification_handler(TauriNotificationHandler::new(app_handle));
    manager
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试时使用 mock 通知处理器
    struct MockNotificationHandler {
        sent: std::sync::Mutex<Vec<(String, String, bool)>>,
    }

    impl MockNotificationHandler {
        fn new() -> Self {
            Self {
                sent: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn sent_count(&self) -> usize {
            self.sent.lock().unwrap().len()
        }
    }

    impl NotificationHandler for MockNotificationHandler {
        fn send_notification(&self, title: &str, body: &str, is_critical: bool) {
            self.sent
                .lock()
                .unwrap()
                .push((title.to_string(), body.to_string(), is_critical));
        }
    }

    #[tokio::test]
    async fn test_mock_notification_handler() {
        let handler = MockNotificationHandler::new();
        handler.send_notification("Test Title", "Test Body", false);
        assert_eq!(handler.sent_count(), 1);
    }
}
