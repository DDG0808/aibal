// IPC Commands 模块
// 定义前端可调用的命令

pub mod events;
pub mod installer;
pub mod ipc;
pub mod plugin;

use tauri::command;

// 导出插件管理器状态
pub use plugin::{create_plugin_manager, PluginManagerState};

// 导出 Phase 2 旧版命令 (向后兼容)
pub use plugin::{
    discover_plugins, disable_plugin as old_disable_plugin,
    enable_plugin as old_enable_plugin, get_plugin, get_plugins_dir,
    list_plugins as old_list_plugins,
};

// 导出 Phase 7.3 IPC Commands (符合 contracts 定义)
pub use ipc::{
    // 7.3.1 插件管理 Commands (9个)
    plugin_list, plugin_enable, plugin_disable, plugin_install,
    plugin_uninstall, plugin_reload, plugin_check_updates, plugin_update, plugin_rollback,
    // 7.3.2 数据 Commands (4个)
    get_all_data, get_plugin_data, refresh_plugin, refresh_all,
    // 7.3.3 配置 Commands (3个)
    get_plugin_config, set_plugin_config, validate_plugin_config,
    // 7.3.4 监控 Commands (2个)
    get_all_health, get_plugin_health,
};

// 导出 IPC Events
pub use events::{event_names, emitter, IpcEventEmitter};

/// 获取应用版本
#[command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 健康检查
#[command]
pub fn health_check() -> bool {
    true
}

// ============================================================================
// Keychain Commands (macOS)
// ============================================================================

#[cfg(target_os = "macos")]
mod keychain {
    use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
    use tauri::command;

    #[command]
    pub fn keychain_set(service: String, key: String, value: String) -> Result<(), String> {
        set_generic_password(&service, &key, value.as_bytes())
            .map_err(|e| format!("Failed to set keychain item: {}", e))
    }

    #[command]
    pub fn keychain_get(service: String, key: String) -> Result<Option<String>, String> {
        match get_generic_password(&service, &key) {
            Ok(password) => {
                String::from_utf8(password.to_vec())
                    .map(Some)
                    .map_err(|e| format!("Failed to decode password: {}", e))
            }
            Err(e) if e.code() == -25300 => Ok(None), // errSecItemNotFound
            Err(e) => Err(format!("Failed to get keychain item: {}", e)),
        }
    }

    #[command]
    pub fn keychain_delete(service: String, key: String) -> Result<(), String> {
        match delete_generic_password(&service, &key) {
            Ok(_) => Ok(()),
            Err(e) if e.code() == -25300 => Ok(()), // errSecItemNotFound
            Err(e) => Err(format!("Failed to delete keychain item: {}", e)),
        }
    }
}

#[cfg(target_os = "macos")]
pub use keychain::*;

// 非 macOS 平台的占位实现
#[cfg(not(target_os = "macos"))]
mod keychain_stub {
    use tauri::command;

    #[command]
    pub fn keychain_set(_service: String, _key: String, _value: String) -> Result<(), String> {
        Err("Keychain is only available on macOS".to_string())
    }

    #[command]
    pub fn keychain_get(_service: String, _key: String) -> Result<Option<String>, String> {
        Err("Keychain is only available on macOS".to_string())
    }

    #[command]
    pub fn keychain_delete(_service: String, _key: String) -> Result<(), String> {
        Err("Keychain is only available on macOS".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
pub use keychain_stub::*;
