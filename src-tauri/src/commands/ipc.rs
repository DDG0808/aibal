// Phase 7.3: IPC Commands
// 实现 contracts/types/ipc-commands.d.ts 定义的 18 个命令

use crate::commands::events::emitter;
use crate::commands::PluginManagerState;
use crate::plugin::types::{
    AppError, PluginData, PluginHealth, PluginInfo, Result as IpcResult, UpdateInfo,
    ValidationResult,
};
use std::collections::HashMap;
use tauri::{command, AppHandle, State};

// ============================================================================
// 7.3.1 插件管理 Commands (9个)
// ============================================================================

/// 获取所有插件列表
#[command]
pub async fn plugin_list(
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Vec<PluginInfo>>, String> {
    let plugins = state.0.list_plugins().await;
    Ok(IpcResult::ok(plugins))
}

/// 启用插件
#[command]
pub async fn plugin_enable(
    app: AppHandle,
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    // PluginManager 内部 RwLock 会处理并发控制
    match state.0.enable_plugin(&id).await {
        Ok(()) => Ok(IpcResult::ok(())),
        Err(e) => {
            let error = AppError::new("PLUGIN_ENABLE_FAILED", e.to_string());
            // 发射错误事件并记录日志（P1: 可观测性）
            if let Err(emit_err) = emitter(&app).emit_plugin_error(&id, &error) {
                log::warn!("发送插件错误事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::err(error))
        }
    }
}

/// 禁用插件
#[command]
pub async fn plugin_disable(
    app: AppHandle,
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    // PluginManager 内部 RwLock 会处理并发控制
    match state.0.disable_plugin(&id).await {
        Ok(()) => Ok(IpcResult::ok(())),
        Err(e) => {
            let error = AppError::new("PLUGIN_DISABLE_FAILED", e.to_string());
            // 发射错误事件并记录日志（P1: 可观测性）
            if let Err(emit_err) = emitter(&app).emit_plugin_error(&id, &error) {
                log::warn!("发送插件错误事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::err(error))
        }
    }
}

/// 安装插件
#[command]
pub async fn plugin_install(
    app: AppHandle,
    source: String,
    skip_signature: Option<bool>,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<PluginInfo>, String> {
    // TODO: Phase 5A 安全验证集成后实现
    // 成功后调用: emitter(&app).emit_plugin_installed(&plugin_info)
    // 失败后调用: emitter(&app).emit_plugin_error(&id, &error)
    let _ = (app, source, skip_signature, state);
    Ok(IpcResult::err(AppError::new(
        "NOT_IMPLEMENTED",
        "插件安装功能尚未实现，需要 Phase 5A 安全验证集成",
    )))
}

/// 卸载插件
#[command]
pub async fn plugin_uninstall(
    app: AppHandle,
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    match state.0.uninstall_plugin(&id).await {
        Ok(()) => {
            if let Err(emit_err) = emitter(&app).emit_plugin_uninstalled(&id) {
                log::warn!("发送插件卸载事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::ok(()))
        }
        Err(e) => {
            let error = AppError::new("PLUGIN_UNINSTALL_FAILED", e.to_string());
            if let Err(emit_err) = emitter(&app).emit_plugin_error(&id, &error) {
                log::warn!("发送插件错误事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::err(error))
        }
    }
}

/// 重载插件
#[command]
pub async fn plugin_reload(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    match state.0.reload_plugin(&id).await {
        Ok(_info) => Ok(IpcResult::ok(())),
        Err(e) => Ok(IpcResult::err(AppError::new("PLUGIN_RELOAD_FAILED", e.to_string()))),
    }
}

/// 检查插件更新
/// 注：完整的更新检查需要远程仓库支持，当前返回空列表
#[command]
pub async fn plugin_check_updates(
    _state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Vec<UpdateInfo>>, String> {
    // 更新检查需要远程仓库支持，当前无远程仓库配置
    Ok(IpcResult::ok(vec![]))
}

/// 更新插件
/// 注：完整的更新功能需要远程仓库支持，当前仅执行重载
#[command]
pub async fn plugin_update(
    app: AppHandle,
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<PluginInfo>, String> {
    // 简化实现：重载插件作为更新操作
    match state.0.reload_plugin(&id).await {
        Ok(info) => {
            if let Err(emit_err) = emitter(&app).emit_plugin_updated(&info) {
                log::warn!("发送插件更新事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::ok(info))
        }
        Err(e) => {
            let error = AppError::new("PLUGIN_UPDATE_FAILED", e.to_string());
            if let Err(emit_err) = emitter(&app).emit_plugin_error(&id, &error) {
                log::warn!("发送插件错误事件失败: plugin={}, emit_error={}", id, emit_err);
            }
            Ok(IpcResult::err(error))
        }
    }
}

/// 回滚插件
/// 注：完整的版本回滚需要版本历史支持，当前不支持
#[command]
pub async fn plugin_rollback(
    id: String,
    version: String,
    _state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    // 版本回滚需要版本历史记录支持，当前未实现版本管理
    log::warn!("插件回滚请求: id={}, version={}, 功能暂不支持", id, version);
    Ok(IpcResult::err(AppError::new(
        "NOT_SUPPORTED",
        "版本回滚需要版本历史记录支持，当前未实现",
    )))
}

// ============================================================================
// 7.3.2 数据 Commands (4个)
// ============================================================================

/// 获取所有插件数据
#[command]
pub async fn get_all_data(
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Vec<PluginData>>, String> {
    let data = state.0.get_all_data().await;
    Ok(IpcResult::ok(data))
}

/// 获取单个插件数据
#[command]
pub async fn get_plugin_data(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Option<PluginData>>, String> {
    let data = state.0.get_plugin_data(&id).await;
    Ok(IpcResult::ok(data))
}

/// 刷新单个插件
/// 注：完整的刷新需要执行插件 JS 代码，当前返回缓存数据
#[command]
pub async fn refresh_plugin(
    app: AppHandle,
    id: String,
    _force: Option<bool>,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Option<PluginData>>, String> {
    // 当前返回缓存数据，完整实现需要 SandboxRuntime 执行插件
    let data = state.0.get_plugin_data(&id).await;
    if let Some(ref d) = data {
        if let Err(emit_err) = emitter(&app).emit_plugin_data_updated(&id, d) {
            log::warn!("发送插件数据更新事件失败: plugin={}, emit_error={}", id, emit_err);
        }
    }
    Ok(IpcResult::ok(data))
}

/// 刷新所有插件
/// 注：完整的刷新需要执行插件 JS 代码，当前返回缓存数据
#[command]
pub async fn refresh_all(
    app: AppHandle,
    _force: Option<bool>,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Vec<PluginData>>, String> {
    // 当前返回缓存数据，完整实现需要 SandboxRuntime 执行插件
    let data = state.0.get_all_data().await;
    for d in &data {
        let plugin_id = match d {
            PluginData::Usage(u) => &u.base.plugin_id,
            PluginData::Balance(b) => &b.base.plugin_id,
            PluginData::Status(s) => &s.base.plugin_id,
            PluginData::Custom(c) => &c.base.plugin_id,
        };
        if let Err(emit_err) = emitter(&app).emit_plugin_data_updated(plugin_id, d) {
            log::warn!("发送插件数据更新事件失败: plugin={}, emit_error={}", plugin_id, emit_err);
        }
    }
    Ok(IpcResult::ok(data))
}

// ============================================================================
// 7.3.3 配置 Commands (3个)
// ============================================================================

/// 获取插件配置
#[command]
pub async fn get_plugin_config(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<HashMap<String, serde_json::Value>>, String> {
    match state.0.get_plugin_config(&id).await {
        Some(config) => Ok(IpcResult::ok(config)),
        None => Ok(IpcResult::err(AppError::new("PLUGIN_NOT_FOUND", format!("插件不存在: {}", id)))),
    }
}

/// 设置插件配置
#[command]
pub async fn set_plugin_config(
    id: String,
    config: HashMap<String, serde_json::Value>,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<()>, String> {
    match state.0.set_plugin_config(&id, config).await {
        Ok(()) => Ok(IpcResult::ok(())),
        Err(e) => Ok(IpcResult::err(AppError::new("CONFIG_SET_FAILED", e.to_string()))),
    }
}

/// 验证插件配置
#[command]
pub async fn validate_plugin_config(
    id: String,
    config: HashMap<String, serde_json::Value>,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<ValidationResult>, String> {
    let result = state.0.validate_plugin_config(&id, &config).await;
    Ok(IpcResult::ok(result))
}

// ============================================================================
// 7.3.4 监控 Commands (2个)
// ============================================================================

/// 获取所有插件健康状态
#[command]
pub async fn get_all_health(
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Vec<PluginHealth>>, String> {
    let health = state.0.get_all_health().await;
    Ok(IpcResult::ok(health))
}

/// 获取单个插件健康状态
#[command]
pub async fn get_plugin_health(
    id: String,
    state: State<'_, PluginManagerState>,
) -> Result<IpcResult<Option<PluginHealth>>, String> {
    let health = state.0.get_plugin_health(&id).await;
    Ok(IpcResult::ok(health))
}
