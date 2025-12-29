// Phase 7.3.5: IPC Events
// 实现 contracts/types/ipc-events.d.ts 定义的 6 个事件

use crate::plugin::types::{AppError, PluginData, PluginHealth, PluginInfo};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

// ============================================================================
// 事件名称常量
// ============================================================================

/// IPC 事件名称
pub mod event_names {
    pub const PLUGIN_INSTALLED: &str = "ipc:plugin_installed";
    pub const PLUGIN_UNINSTALLED: &str = "ipc:plugin_uninstalled";
    pub const PLUGIN_UPDATED: &str = "ipc:plugin_updated";
    pub const PLUGIN_DATA_UPDATED: &str = "ipc:plugin_data_updated";
    pub const PLUGIN_ERROR: &str = "ipc:plugin_error";
    pub const PLUGIN_HEALTH_CHANGED: &str = "ipc:plugin_health_changed";
}

// ============================================================================
// 事件 Payload 类型
// ============================================================================

/// 插件卸载事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct PluginUninstalledPayload {
    pub id: String,
}

/// 插件数据更新事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct PluginDataUpdatedPayload {
    pub id: String,
    pub data: PluginData,
}

/// 插件错误事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct PluginErrorPayload {
    pub id: String,
    pub error: AppError,
}

// ============================================================================
// 事件发射器
// ============================================================================

/// IPC 事件发射器
/// 提供类型安全的事件发射方法
pub struct IpcEventEmitter<'a> {
    app: &'a AppHandle,
}

impl<'a> IpcEventEmitter<'a> {
    /// 创建事件发射器
    pub fn new(app: &'a AppHandle) -> Self {
        Self { app }
    }

    /// 发送插件安装完成事件
    pub fn emit_plugin_installed(&self, info: &PluginInfo) -> Result<(), tauri::Error> {
        self.app.emit(event_names::PLUGIN_INSTALLED, info)
    }

    /// 发送插件卸载完成事件
    pub fn emit_plugin_uninstalled(&self, id: &str) -> Result<(), tauri::Error> {
        self.app.emit(
            event_names::PLUGIN_UNINSTALLED,
            PluginUninstalledPayload { id: id.to_string() },
        )
    }

    /// 发送插件更新完成事件
    pub fn emit_plugin_updated(&self, info: &PluginInfo) -> Result<(), tauri::Error> {
        self.app.emit(event_names::PLUGIN_UPDATED, info)
    }

    /// 发送插件数据更新事件
    pub fn emit_plugin_data_updated(&self, id: &str, data: &PluginData) -> Result<(), tauri::Error> {
        self.app.emit(
            event_names::PLUGIN_DATA_UPDATED,
            PluginDataUpdatedPayload {
                id: id.to_string(),
                data: data.clone(),
            },
        )
    }

    /// 发送插件错误事件
    pub fn emit_plugin_error(&self, id: &str, error: &AppError) -> Result<(), tauri::Error> {
        self.app.emit(
            event_names::PLUGIN_ERROR,
            PluginErrorPayload {
                id: id.to_string(),
                error: error.clone(),
            },
        )
    }

    /// 发送插件健康状态变化事件
    pub fn emit_plugin_health_changed(&self, health: &PluginHealth) -> Result<(), tauri::Error> {
        self.app.emit(event_names::PLUGIN_HEALTH_CHANGED, health)
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 创建事件发射器的便捷函数
pub fn emitter(app: &AppHandle) -> IpcEventEmitter<'_> {
    IpcEventEmitter::new(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_names() {
        assert_eq!(event_names::PLUGIN_INSTALLED, "ipc:plugin_installed");
        assert_eq!(event_names::PLUGIN_UNINSTALLED, "ipc:plugin_uninstalled");
        assert_eq!(event_names::PLUGIN_UPDATED, "ipc:plugin_updated");
        assert_eq!(event_names::PLUGIN_DATA_UPDATED, "ipc:plugin_data_updated");
        assert_eq!(event_names::PLUGIN_ERROR, "ipc:plugin_error");
        assert_eq!(event_names::PLUGIN_HEALTH_CHANGED, "ipc:plugin_health_changed");
    }
}
