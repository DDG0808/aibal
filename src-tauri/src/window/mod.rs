// Phase 7.2: 窗口管理
// 实现多窗口创建和状态同步

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

// ============================================================================
// 窗口类型
// ============================================================================

/// 窗口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowType {
    /// 托盘弹窗 (主面板)
    Popup,
    /// 仪表盘窗口 (主应用)
    Dashboard,
    /// 设置窗口
    Settings,
    /// 首次设置向导
    Wizard,
    /// 关于窗口
    About,
}

/// 窗口配置
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub label: &'static str,
    pub title: &'static str,
    pub url: &'static str,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub center: bool,
    /// 是否在任务栏/Dock 中隐藏
    pub skip_taskbar: bool,
    /// macOS 标题栏样式
    pub title_bar_style: Option<tauri::TitleBarStyle>,
    /// 是否隐藏标题
    pub hidden_title: bool,
}

impl WindowType {
    /// 获取窗口配置
    pub fn config(&self) -> WindowConfig {
        match self {
            WindowType::Popup => WindowConfig {
                label: "main",
                title: "CUK",
                url: "/home",
                width: 300.0,
                height: 360.0,
                resizable: false,
                decorations: false,
                transparent: true,
                always_on_top: true,
                center: false,
                skip_taskbar: true,
                title_bar_style: None,
                hidden_title: true,
            },
            WindowType::Dashboard => WindowConfig {
                label: "dashboard",
                title: "CUK",
                url: "/dashboard",
                width: 900.0,
                height: 650.0,
                resizable: true,
                decorations: true,
                transparent: false,
                always_on_top: false,
                center: true,
                skip_taskbar: false,
                // Overlay 模式：内容延伸到标题栏，traffic lights 覆盖在内容上
                title_bar_style: Some(tauri::TitleBarStyle::Overlay),
                hidden_title: true,
            },
            WindowType::Settings => WindowConfig {
                label: "settings",
                title: "CUK 设置",
                url: "/settings",
                width: 800.0,
                height: 600.0,
                resizable: true,
                decorations: true,
                transparent: false,
                always_on_top: false,
                center: true,
                skip_taskbar: false,
                title_bar_style: None,
                hidden_title: false,
            },
            WindowType::Wizard => WindowConfig {
                label: "wizard",
                title: "欢迎使用 CUK",
                url: "/wizard",
                width: 600.0,
                height: 400.0,
                resizable: false,
                decorations: true,
                transparent: false,
                always_on_top: true,
                center: true,
                skip_taskbar: true,
                title_bar_style: None,
                hidden_title: false,
            },
            WindowType::About => WindowConfig {
                label: "about",
                title: "关于 CUK",
                url: "/about",
                width: 400.0,
                height: 300.0,
                resizable: false,
                decorations: true,
                transparent: false,
                always_on_top: true,
                center: true,
                skip_taskbar: true,
                title_bar_style: None,
                hidden_title: false,
            },
        }
    }
}

// ============================================================================
// 窗口管理器
// ============================================================================

/// 窗口管理器
pub struct WindowManager;

impl WindowManager {
    /// 创建或显示窗口
    pub fn open<R: Runtime>(app: &AppHandle<R>, window_type: WindowType) -> Option<WebviewWindow<R>> {
        let config = window_type.config();

        // 检查窗口是否已存在
        if let Some(window) = app.get_webview_window(config.label) {
            if let Err(e) = window.show() {
                log::warn!("显示已有窗口失败: label={}, error={}", config.label, e);
            }
            if let Err(e) = window.set_focus() {
                log::warn!("设置窗口焦点失败: label={}, error={}", config.label, e);
            }
            return Some(window);
        }

        // 创建新窗口
        let mut builder = tauri::WebviewWindowBuilder::new(
            app,
            config.label,
            tauri::WebviewUrl::App(config.url.into()),
        )
        .title(config.title)
        .inner_size(config.width, config.height)
        .resizable(config.resizable)
        .decorations(config.decorations)
        .transparent(config.transparent)
        .always_on_top(config.always_on_top)
        .skip_taskbar(config.skip_taskbar)
        .hidden_title(config.hidden_title)
        .visible(true);

        // macOS 标题栏样式
        if let Some(style) = config.title_bar_style {
            builder = builder.title_bar_style(style);
        }

        if config.center {
            builder = builder.center();
        }

        match builder.build() {
            Ok(window) => {
                log::info!("窗口已创建: {}", config.label);

                Some(window)
            }
            Err(e) => {
                log::error!("创建窗口失败: {}", e);
                None
            }
        }
    }

    /// 关闭窗口
    pub fn close<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
        if let Some(window) = app.get_webview_window(label) {
            match window.close() {
                Ok(_) => {
                    log::info!("窗口已关闭: {}", label);
                    true
                }
                Err(e) => {
                    log::error!("关闭窗口失败: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// 隐藏窗口
    pub fn hide<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
        if let Some(window) = app.get_webview_window(label) {
            match window.hide() {
                Ok(_) => true,
                Err(e) => {
                    log::warn!("隐藏窗口失败: label={}, error={}", label, e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// 显示窗口
    pub fn show<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
        if let Some(window) = app.get_webview_window(label) {
            if let Err(e) = window.show() {
                log::warn!("显示窗口失败: label={}, error={}", label, e);
                return false;
            }
            if let Err(e) = window.set_focus() {
                log::warn!("设置窗口焦点失败: label={}, error={}", label, e);
            }
            true
        } else {
            false
        }
    }

    /// 打开仪表盘窗口并导航到指定路由
    pub fn open_dashboard_with_route<R: Runtime>(
        app: &AppHandle<R>,
        route: Option<&str>,
    ) -> Option<WebviewWindow<R>> {
        let config = WindowType::Dashboard.config();
        let target_route = route.unwrap_or("/dashboard");

        // 检查窗口是否已存在
        if let Some(window) = app.get_webview_window(config.label) {
            if let Err(e) = window.show() {
                log::warn!("显示已有窗口失败: label={}, error={}", config.label, e);
            }
            if let Err(e) = window.set_focus() {
                log::warn!("设置窗口焦点失败: label={}, error={}", config.label, e);
            }
            // 向窗口发送导航事件
            if let Err(e) = window.emit("navigate", target_route) {
                log::warn!("发送导航事件失败: route={}, error={}", target_route, e);
            }
            return Some(window);
        }

        // 创建新窗口（带初始路由）
        let url = if target_route != "/dashboard" {
            target_route
        } else {
            config.url
        };

        let mut builder = tauri::WebviewWindowBuilder::new(
            app,
            config.label,
            tauri::WebviewUrl::App(url.into()),
        )
        .title(config.title)
        .inner_size(config.width, config.height)
        .resizable(config.resizable)
        .decorations(config.decorations)
        .transparent(config.transparent)
        .always_on_top(config.always_on_top)
        .skip_taskbar(config.skip_taskbar)
        .hidden_title(config.hidden_title)
        .visible(true)
        .center();

        // macOS 标题栏样式
        if let Some(style) = config.title_bar_style {
            builder = builder.title_bar_style(style);
        }

        match builder.build() {
            Ok(window) => {
                log::info!("仪表盘窗口已创建: route={}", url);
                Some(window)
            }
            Err(e) => {
                log::error!("创建仪表盘窗口失败: {}", e);
                None
            }
        }
    }
}

// ============================================================================
// 多窗口状态同步
// ============================================================================

/// 窗口状态同步事件名称
pub mod sync_events {
    pub const STATE_CHANGED: &str = "window:state_changed";
    pub const THEME_CHANGED: &str = "window:theme_changed";
    pub const DATA_UPDATED: &str = "window:data_updated";
}

/// 状态变化事件 Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangedPayload {
    /// 状态类型
    pub state_type: String,
    /// 状态数据
    pub data: serde_json::Value,
    /// 来源窗口
    pub source_window: Option<String>,
}

/// 广播状态变化到所有窗口
pub fn broadcast_state<R: Runtime>(
    app: &AppHandle<R>,
    state_type: &str,
    data: serde_json::Value,
    source_window: Option<&str>,
) -> Result<(), tauri::Error> {
    let payload = StateChangedPayload {
        state_type: state_type.to_string(),
        data,
        source_window: source_window.map(String::from),
    };

    app.emit(sync_events::STATE_CHANGED, &payload)?;
    log::debug!("状态广播: {} (来源: {:?})", state_type, source_window);
    Ok(())
}

/// 广播主题变化
pub fn broadcast_theme<R: Runtime>(
    app: &AppHandle<R>,
    theme: &str,
) -> Result<(), tauri::Error> {
    app.emit(sync_events::THEME_CHANGED, theme)?;
    log::debug!("主题变化广播: {}", theme);
    Ok(())
}

/// 广播数据更新
pub fn broadcast_data_update<R: Runtime>(
    app: &AppHandle<R>,
    data: serde_json::Value,
) -> Result<(), tauri::Error> {
    app.emit(sync_events::DATA_UPDATED, &data)?;
    log::debug!("数据更新广播");
    Ok(())
}

// ============================================================================
// 首次设置向导
// ============================================================================

/// 检查是否需要显示首次设置向导
pub fn should_show_wizard() -> bool {
    // TODO: 检查配置文件是否存在，或者用户是否已完成首次设置
    // 目前暂时返回 false
    false
}

/// 显示首次设置向导
pub fn show_wizard<R: Runtime>(app: &AppHandle<R>) {
    if should_show_wizard() {
        WindowManager::open(app, WindowType::Wizard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_type_config() {
        let popup = WindowType::Popup.config();
        assert_eq!(popup.label, "main");
        assert!(!popup.decorations);
        assert!(popup.transparent);

        let settings = WindowType::Settings.config();
        assert_eq!(settings.label, "settings");
        assert!(settings.decorations);
        assert!(settings.resizable);
    }
}
