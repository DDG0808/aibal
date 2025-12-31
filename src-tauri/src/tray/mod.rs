// Phase 7.1: 系统托盘
// 实现 macOS 菜单栏应用的托盘功能

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, Wry,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::window::{WindowManager, WindowType};

// ============================================================================
// 托盘状态
// ============================================================================

/// 托盘状态 (用于动态图标着色)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    /// 正常状态 (默认图标)
    Normal,
    /// 警告状态 (黄色/橙色图标)
    Warning,
    /// 错误状态 (红色图标)
    Error,
    /// 加载中 (可选的加载动画)
    Loading,
}

// ============================================================================
// 托盘管理器
// ============================================================================

/// 托盘管理器
pub struct TrayManager {
    /// 当前状态
    status: TrayStatus,
}

impl TrayManager {
    /// 创建新的托盘管理器
    pub fn new() -> Self {
        Self {
            status: TrayStatus::Normal,
        }
    }

    /// 获取当前状态
    pub fn status(&self) -> TrayStatus {
        self.status
    }

    /// 设置状态
    pub fn set_status(&mut self, status: TrayStatus) {
        self.status = status;
    }
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 托盘设置函数
// ============================================================================

/// 设置系统托盘
///
/// 注意: tauri.conf.json 已配置 trayIcon，Tauri 会自动创建托盘。
/// 此函数获取已有托盘并设置菜单和事件处理器。
pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), tauri::Error> {
    // 创建托盘菜单
    let menu = create_tray_menu(app)?;

    // 尝试获取已由 tauri.conf.json 创建的托盘
    if let Some(tray) = app.tray_by_id("main") {
        // 设置菜单
        tray.set_menu(Some(menu))?;
        // 设置左键点击不显示菜单
        tray.set_show_menu_on_left_click(false)?;
        // 设置工具提示
        tray.set_tooltip(Some("CUK - Claude Usage Tracker"))?;

        // 注册菜单事件处理器
        tray.on_menu_event(|app, event| {
            handle_menu_event(app, &event.id.0);
        });

        // 注册托盘图标事件处理器
        tray.on_tray_icon_event(|tray, event| {
            // 调用 positioner 插件处理托盘位置状态
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            handle_tray_event(tray, event);
        });

        log::info!("系统托盘已配置 (使用 tauri.conf.json 创建的托盘)");
        return Ok(());
    }

    // 如果没有预创建的托盘，则手动创建（兼容模式）
    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        tauri::Error::AssetNotFound("default window icon".to_string())
    })?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("CUK - Claude Usage Tracker")
        .on_menu_event(|app, event| {
            handle_menu_event(app, &event.id.0);
        })
        .on_tray_icon_event(|tray, event| {
            // 调用 positioner 插件处理托盘位置状态
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            handle_tray_event(tray, event);
        })
        .build(app)?;

    log::info!("系统托盘已创建 (手动创建)");
    Ok(())
}

/// 创建托盘菜单
fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, tauri::Error> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "打开主面板", true, None::<&str>)?,
            &MenuItem::with_id(app, "refresh", "刷新数据", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "settings", "设置...", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "about", "关于 CUK", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
        ],
    )?;

    Ok(menu)
}

/// 处理菜单事件
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    log::debug!("托盘菜单点击: {}", menu_id);

    match menu_id {
        "open" => {
            // 打开主窗口
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.show() {
                    log::warn!("显示主窗口失败: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::warn!("设置窗口焦点失败: {}", e);
                }
            }
        }
        "refresh" => {
            // 发送刷新事件
            if let Err(e) = app.emit("tray:refresh", ()) {
                log::warn!("发送刷新事件失败: {}", e);
            } else {
                log::info!("触发数据刷新");
            }
        }
        "settings" => {
            // 打开设置窗口 (使用 WindowManager 统一管理)
            WindowManager::open(app, WindowType::Settings);
        }
        "about" => {
            // 打开关于窗口 (使用 WindowManager 统一管理)
            WindowManager::open(app, WindowType::About);
        }
        "quit" => {
            // 退出应用
            log::info!("用户请求退出");
            app.exit(0);
        }
        _ => {
            log::warn!("未知菜单项: {}", menu_id);
        }
    }
}

/// 处理托盘图标事件
fn handle_tray_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            position,
            rect,
            ..
        } => {
            // 左键点击 - 切换主窗口显示
            // 传递托盘图标的位置和尺寸用于精确定位
            log::debug!("托盘左键点击: position={:?}, rect={:?}", position, rect);
            toggle_main_window(tray.app_handle(), Some((position, rect)));
        }
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            position,
            rect,
            ..
        } => {
            // 左键双击 - 显示主窗口
            log::debug!("托盘左键双击");
            if let Some(window) = tray.app_handle().get_webview_window("main") {
                position_window_near_tray(&window, Some((position, rect)));
                if let Err(e) = window.show() {
                    log::warn!("显示主窗口失败: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::warn!("设置窗口焦点失败: {}", e);
                }
            }
        }
        _ => {}
    }
}

/// 托盘位置信息类型
type TrayPosition = (tauri::PhysicalPosition<f64>, tauri::Rect);

/// 切换主窗口显示状态
fn toggle_main_window<R: Runtime>(app: &AppHandle<R>, tray_info: Option<TrayPosition>) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => {
                if let Err(e) = window.hide() {
                    log::warn!("隐藏主窗口失败: {}", e);
                }
            }
            Ok(false) => {
                // 定位窗口到托盘图标下方
                position_window_near_tray(&window, tray_info);
                if let Err(e) = window.show() {
                    log::warn!("显示主窗口失败: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::warn!("设置窗口焦点失败: {}", e);
                }
                // 发送刷新事件，确保托盘弹框显示最新数据
                if let Err(e) = app.emit("tray:refresh", ()) {
                    log::warn!("发送托盘刷新事件失败: {}", e);
                } else {
                    log::debug!("托盘弹框显示，触发数据刷新");
                }
            }
            Err(e) => {
                log::error!("获取窗口可见性失败: {}", e);
            }
        }
    }
}

/// 窗口与菜单栏的间距
const WINDOW_PADDING: f64 = 4.0;

/// 将窗口定位到托盘图标附近
///
/// 多显示器精确定位策略:
/// 1. 使用托盘点击事件提供的精确位置 (包含正确的屏幕坐标)
/// 2. 将窗口水平居中对齐到托盘图标
/// 3. 窗口顶部紧贴菜单栏下方
fn position_window_near_tray<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    tray_info: Option<TrayPosition>,
) {
    // 获取窗口大小
    let window_size = match window.outer_size() {
        Ok(size) => size,
        Err(e) => {
            log::error!("获取窗口大小失败: {}", e);
            return;
        }
    };

    if let Some((_click_pos, tray_rect)) = tray_info {
        // 使用托盘事件提供的精确位置
        // tray_rect.position 和 tray_rect.size 是 DPI-aware 的枚举类型
        // 需要转换为物理像素 (使用 scale_factor = 1.0，因为托盘事件通常返回物理坐标)

        // 转换为物理坐标
        let tray_pos: tauri::PhysicalPosition<i32> = tray_rect.position.to_physical(1.0);
        let tray_size: tauri::PhysicalSize<u32> = tray_rect.size.to_physical(1.0);

        // 计算窗口位置：水平居中对齐托盘图标，垂直紧贴菜单栏下方
        let tray_center_x = tray_pos.x as f64 + (tray_size.width as f64 / 2.0);
        let window_x = tray_center_x - (window_size.width as f64 / 2.0);

        // 窗口 Y 坐标：托盘图标底部 + 间距
        let window_y = tray_pos.y as f64 + tray_size.height as f64 + WINDOW_PADDING;

        log::debug!(
            "多显示器定位: tray=({}, {}, {}x{}), window=({:.0}, {:.0})",
            tray_pos.x, tray_pos.y,
            tray_size.width, tray_size.height,
            window_x, window_y
        );

        // 设置窗口位置 (使用物理像素)
        if let Err(e) = window.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(window_x as i32, window_y as i32),
        )) {
            log::warn!("设置窗口位置失败: {}", e);
            // 回退到 positioner
            fallback_position(window);
        }
    } else {
        // 没有托盘位置信息，使用 positioner 插件
        fallback_position(window);
    }
}

/// 使用 positioner 插件作为回退定位方案
fn fallback_position<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    if let Err(e) = window.move_window_constrained(Position::TrayBottomCenter) {
        log::warn!("positioner TrayBottomCenter 失败: {}", e);
        if let Err(e2) = window.move_window(Position::TrayCenter) {
            log::error!("positioner TrayCenter 也失败: {}", e2);
        }
    }
}

// ============================================================================
// 动态图标更新
// ============================================================================

/// 更新托盘图标状态
///
/// 注意: 当前使用默认图标。如需动态图标，需要准备不同状态的图标资源。
pub fn update_tray_status(app: &AppHandle<Wry>, status: TrayStatus) -> Result<(), tauri::Error> {
    // 获取托盘图标
    if let Some(tray) = app.tray_by_id("main") {
        // 根据状态更新工具提示
        let tooltip = match status {
            TrayStatus::Normal => "CUK - 正常",
            TrayStatus::Warning => "CUK - 警告",
            TrayStatus::Error => "CUK - 错误",
            TrayStatus::Loading => "CUK - 加载中...",
        };
        tray.set_tooltip(Some(tooltip))?;

        // TODO: 动态图标需要在 icons/ 目录准备不同状态的图标
        // 然后使用 tauri::image::Image 加载
        log::debug!("托盘状态更新: {:?}", status);
    }

    Ok(())
}

/// 更新托盘工具提示
pub fn update_tray_tooltip(app: &AppHandle<Wry>, tooltip: &str) -> Result<(), tauri::Error> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_tooltip(Some(tooltip))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_manager() {
        let mut manager = TrayManager::new();
        assert_eq!(manager.status(), TrayStatus::Normal);

        manager.set_status(TrayStatus::Warning);
        assert_eq!(manager.status(), TrayStatus::Warning);
    }
}
