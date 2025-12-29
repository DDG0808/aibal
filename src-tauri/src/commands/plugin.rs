// 插件 IPC Commands
// Phase 2: 插件运行时核心

use crate::plugin::{PluginDiscovery, PluginManager};
use crate::plugin::types::PluginInfo;
use std::sync::Arc;
use tauri::{command, State};

/// 插件管理器状态包装
///
/// 注意：移除了外层 RwLock，因为 PluginManager 内部已有 RwLock<HashMap<...>> 保护。
/// 外层锁是多余的，会降低并发性能并增加死锁风险。
pub struct PluginManagerState(pub Arc<PluginManager>);

/// 列出所有插件
#[command]
pub async fn list_plugins(state: State<'_, PluginManagerState>) -> Result<Vec<PluginInfo>, String> {
    Ok(state.0.list_plugins().await)
}

/// 获取单个插件信息
#[command]
pub async fn get_plugin(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<Option<PluginInfo>, String> {
    Ok(state.0.get_plugin(&id).await)
}

/// 启用插件
#[command]
pub async fn enable_plugin(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<(), String> {
    // PluginManager 内部 RwLock 会处理并发控制
    state.0
        .enable_plugin(&id)
        .await
        .map_err(|e| format!("Failed to enable plugin: {}", e))
}

/// 禁用插件
#[command]
pub async fn disable_plugin(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<(), String> {
    // PluginManager 内部 RwLock 会处理并发控制
    state.0
        .disable_plugin(&id)
        .await
        .map_err(|e| format!("Failed to disable plugin: {}", e))
}

/// 发现并加载插件
#[command]
pub async fn discover_plugins(
    state: State<'_, PluginManagerState>,
) -> Result<Vec<PluginInfo>, String> {
    // PluginManager 内部 RwLock 会处理并发控制
    state.0
        .discover_and_load()
        .await
        .map_err(|e| format!("Failed to discover plugins: {}", e))
}

/// 获取插件目录路径
#[command]
pub async fn get_plugins_dir(
    state: State<'_, PluginManagerState>,
) -> Result<String, String> {
    Ok(state.0.plugins_dir().to_string_lossy().to_string())
}

/// 创建默认的 PluginManager
pub fn create_plugin_manager() -> PluginManagerState {
    let discovery = PluginDiscovery::with_default_dir();
    let manager = PluginManager::new(discovery);
    PluginManagerState(Arc::new(manager))
}
