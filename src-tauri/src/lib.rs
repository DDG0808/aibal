// CUK - Claude Usage Tracker
// macOS 菜单栏应用，用于追踪 Claude AI 使用量

use tauri::Manager;

mod commands;
mod plugin;
mod reliability;
mod security;
mod state;
mod tray;
mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            // 基础命令
            commands::get_version,
            commands::health_check,
            commands::keychain_set,
            commands::keychain_get,
            commands::keychain_delete,
            // Phase 2 插件命令 (向后兼容)
            crate::commands::plugin::list_plugins,
            crate::commands::plugin::get_plugin,
            crate::commands::plugin::enable_plugin,
            crate::commands::plugin::disable_plugin,
            crate::commands::plugin::discover_plugins,
            crate::commands::plugin::get_plugins_dir,
            // Phase 7.3.1 插件管理 Commands (9个)
            crate::commands::ipc::plugin_list,
            crate::commands::ipc::plugin_enable,
            crate::commands::ipc::plugin_disable,
            crate::commands::ipc::plugin_install,
            crate::commands::ipc::plugin_uninstall,
            crate::commands::ipc::plugin_reload,
            crate::commands::ipc::plugin_check_updates,
            crate::commands::ipc::plugin_update,
            crate::commands::ipc::plugin_rollback,
            // Phase 7.3.2 数据 Commands (4个)
            crate::commands::ipc::get_all_data,
            crate::commands::ipc::get_plugin_data,
            crate::commands::ipc::refresh_plugin,
            crate::commands::ipc::refresh_all,
            // Phase 7.3.3 配置 Commands (3个)
            crate::commands::ipc::get_plugin_config,
            crate::commands::ipc::set_plugin_config,
            crate::commands::ipc::validate_plugin_config,
            // Phase 7.3.4 监控 Commands (2个)
            crate::commands::ipc::get_all_health,
            crate::commands::ipc::get_plugin_health,
        ])
        .setup(|app| {
            // 初始化日志
            log::info!("CUK 应用启动中...");

            // 设置 macOS 激活策略为 accessory (隐藏 Dock 图标)
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // 初始化插件管理器 (Phase 2)
            let plugin_manager = commands::create_plugin_manager();

            // Phase 4 修复：调用 init() 启动分发器
            // init() 包含：discover_and_load + start_dispatcher + start_call_dispatcher
            let manager_for_init = plugin_manager.0.clone();
            tauri::async_runtime::spawn(async move {
                match manager_for_init.init().await {
                    Ok(plugins) => {
                        log::info!(
                            "插件系统初始化完成: {} 个插件, EventBus/Call 分发器已启动",
                            plugins.len()
                        );
                        for plugin in plugins {
                            log::debug!("  - {} v{} ({})", plugin.name, plugin.version, plugin.id);
                        }
                    }
                    Err(e) => {
                        log::error!("插件系统初始化失败: {}", e);
                    }
                }
            });

            app.manage(plugin_manager);
            log::info!("插件管理器已创建");

            // 初始化系统托盘 (Phase 7.1)
            match tray::setup_tray(app.handle()) {
                Ok(_) => log::info!("系统托盘已初始化"),
                Err(e) => log::error!("系统托盘初始化失败: {}", e),
            }

            // 初始化托盘管理器
            app.manage(tray::TrayManager::new());

            // 获取主窗口引用
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                // 默认隐藏主窗口，通过托盘点击显示
                if let Err(e) = window.hide() {
                    log::warn!("隐藏主窗口失败: {}", e);
                }
                log::info!("主窗口已创建 (隐藏): {:?}", window.label());
            }

            log::info!("CUK 应用启动完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
