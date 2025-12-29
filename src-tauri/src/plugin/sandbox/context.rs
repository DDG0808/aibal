// 插件上下文 API 模块
// Phase 4: 通信与配置
//
// 提供给 JS 插件使用的 context API:
// - context.emit(event, data) - 发布事件
// - context.call(pluginId, method, params) - 跨插件调用
// - context.config - 插件配置（只读）
// - context.pluginId - 当前插件 ID

use std::collections::HashMap;
use std::sync::Arc;

use rquickjs::prelude::Rest;
use rquickjs::{Ctx, Function, IntoJs, Object, Result as JsResult, Value};
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;

use crate::plugin::event_bus::EventBus;
use crate::plugin::permission::PermissionChecker;

// ============================================================================
// 跨插件调用处理器 Trait
// ============================================================================

/// 跨插件调用请求
#[derive(Debug)]
pub struct PluginCallRequest {
    /// 调用方插件 ID
    pub caller: String,
    /// 目标插件 ID
    pub target: String,
    /// 方法名
    pub method: String,
    /// 参数
    pub params: JsonValue,
    /// 当前调用深度（从 1 开始，最大为 MAX_CALL_DEPTH）
    pub call_depth: usize,
    /// 响应通道
    pub response_tx: tokio::sync::oneshot::Sender<Result<JsonValue, String>>,
}

impl PluginCallRequest {
    /// 最大调用深度
    pub const MAX_CALL_DEPTH: usize = 3;
}

/// 事件发布请求
#[derive(Debug)]
pub struct EmitRequest {
    /// 发布者插件 ID
    pub plugin_id: String,
    /// 事件 action
    pub action: String,
    /// 事件数据
    pub data: JsonValue,
}

// ============================================================================
// 插件上下文配置
// ============================================================================

/// 插件上下文配置
pub struct PluginContextConfig {
    /// 插件 ID
    pub plugin_id: String,
    /// 事件总线
    pub event_bus: Arc<EventBus>,
    /// 权限检查器
    pub permission_checker: Arc<PermissionChecker>,
    /// 插件配置
    pub config: HashMap<String, JsonValue>,
    /// 跨插件调用请求发送端
    pub call_tx: mpsc::Sender<PluginCallRequest>,
}

// ============================================================================
// 插件上下文 API
// ============================================================================

/// 插件上下文 API 初始化器
pub struct PluginContextApi;

impl PluginContextApi {
    /// 注入 context 对象到 JS 全局作用域
    ///
    /// 注入的 API:
    /// - `context.pluginId` - 当前插件 ID (string)
    /// - `context.config` - 插件配置 (object, 只读, 已冻结)
    /// - `context.emit(event, data)` - 发布事件 (同步，入队)
    /// - `context.call(pluginId, method, params)` - 跨插件调用 (返回 JSON 字符串)
    ///   - 返回格式: `{"success": boolean, "status": string, "message": string}`
    ///   - 使用示例: `const result = JSON.parse(context.call(...))`
    ///
    /// # 参数
    /// - `ctx`: JS 上下文
    /// - `config`: 插件上下文配置
    pub fn inject(ctx: &Ctx<'_>, config: PluginContextConfig) -> JsResult<()> {
        let globals = ctx.globals();

        // 创建 context 对象
        let context_obj = Object::new(ctx.clone())?;

        // 1. 注入 pluginId
        context_obj.set("pluginId", config.plugin_id.clone())?;

        // 2. 注入 config（只读，使用 Object.freeze 冻结）
        let config_obj = Self::config_to_js(ctx, &config.config)?;
        // 调用 Object.freeze 冻结 config 对象
        let object_ctor: Object = globals.get("Object")?;
        let freeze_fn: Function = object_ctor.get("freeze")?;
        let frozen_config: Value = freeze_fn.call((config_obj.clone(),))?;
        context_obj.set("config", frozen_config)?;

        // 3. 注入 emit 函数
        let emit_fn = Self::create_emit_function(
            ctx,
            config.plugin_id.clone(),
            config.event_bus.clone(),
        )?;
        context_obj.set("emit", emit_fn)?;

        // 4. 注入 call 函数
        let call_fn = Self::create_call_function(
            ctx,
            config.plugin_id.clone(),
            config.permission_checker.clone(),
            config.call_tx.clone(),
        )?;
        context_obj.set("call", call_fn)?;

        // 5. 注入 log 函数（便捷方法，带插件 ID 前缀）
        let log_fn = Self::create_log_function(ctx, config.plugin_id.clone())?;
        context_obj.set("log", log_fn)?;

        // 设置到全局
        globals.set("context", context_obj)?;

        log::debug!("已注入 context API: plugin_id={}", config.plugin_id);
        Ok(())
    }

    /// 将 HashMap 配置转换为 JS 对象
    fn config_to_js<'js>(
        ctx: &Ctx<'js>,
        config: &HashMap<String, JsonValue>,
    ) -> JsResult<Object<'js>> {
        let obj = Object::new(ctx.clone())?;

        for (key, value) in config {
            let js_value = Self::json_to_js(ctx, value)?;
            obj.set(key.as_str(), js_value)?;
        }

        Ok(obj)
    }

    /// 将 serde_json::Value 转换为 JS Value
    fn json_to_js<'js>(ctx: &Ctx<'js>, value: &JsonValue) -> JsResult<Value<'js>> {
        match value {
            JsonValue::Null => Ok(Value::new_null(ctx.clone())),
            JsonValue::Bool(b) => b.into_js(ctx),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i.into_js(ctx)
                } else if let Some(f) = n.as_f64() {
                    f.into_js(ctx)
                } else {
                    Ok(Value::new_null(ctx.clone()))
                }
            }
            JsonValue::String(s) => s.as_str().into_js(ctx),
            JsonValue::Array(arr) => {
                let js_arr = rquickjs::Array::new(ctx.clone())?;
                for (i, item) in arr.iter().enumerate() {
                    let js_item = Self::json_to_js(ctx, item)?;
                    js_arr.set(i, js_item)?;
                }
                js_arr.into_js(ctx)
            }
            JsonValue::Object(map) => {
                let obj = Object::new(ctx.clone())?;
                for (k, v) in map {
                    let js_v = Self::json_to_js(ctx, v)?;
                    obj.set(k.as_str(), js_v)?;
                }
                obj.into_js(ctx)
            }
        }
    }

    /// 创建 emit 函数
    ///
    /// `context.emit(event, data)` - 发布事件
    /// - event: 事件 action（不含前缀，会自动添加 `plugin:{pluginId}:` 前缀）
    /// - data: 事件数据（当前简化为 String，待后续支持复杂对象）
    ///
    /// 失败时抛出 JS 异常，不再静默失败
    fn create_emit_function<'js>(
        ctx: &Ctx<'js>,
        plugin_id: String,
        event_bus: Arc<EventBus>,
    ) -> JsResult<Function<'js>> {
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'_>, event: String, data: Option<String>| -> JsResult<()> {
                // 简化版本：data 作为字符串处理
                let json_data = match data {
                    Some(s) => serde_json::json!({ "message": s }),
                    None => JsonValue::Null,
                };

                // 发布事件（同步入队，使用 emit_sync 避免 async）
                match event_bus.emit_sync(&plugin_id, &event, json_data.clone()) {
                    Ok(()) => {
                        log::debug!(
                            "[{}] emit event: plugin:{}:{} data={:?}",
                            plugin_id,
                            plugin_id,
                            event,
                            json_data
                        );
                        Ok(())
                    }
                    Err(e) => {
                        log::warn!("[{}] emit failed: {}", plugin_id, e);
                        // 抛出 JS 异常而非静默失败
                        Err(ctx.throw(rquickjs::Value::from_exception(
                            rquickjs::Exception::from_message(
                                ctx.clone(),
                                &format!("emit failed: {}", e),
                            )?,
                        )))
                    }
                }
            },
        )
    }

    /// 创建 call 函数（占位实现）
    ///
    /// `context.call(pluginId, method, params)` - 跨插件调用
    /// - pluginId: 目标插件 ID
    /// - method: 方法名
    /// - params: 参数（当前未使用）
    ///
    /// **当前状态：功能未实现**
    ///
    /// 返回 JSON 字符串格式:
    /// `{"success": false, "status": "not_supported", "message": "...", "target": "...", "method": "..."}`
    ///
    /// JS 端使用示例:
    /// ```javascript
    /// const result = JSON.parse(context.call('target', 'method', 'params'));
    /// if (!result.success && result.status === 'not_supported') {
    ///     console.log('跨插件调用功能未实现');
    /// }
    /// ```
    ///
    /// 架构限制：
    /// - 当前插件沙盒是"执行后销毁"模式，不支持持久方法调用
    /// - rquickjs 0.6 不支持异步 Promise，无法等待远程结果
    /// - 完整实现需要"常驻沙盒"模式或方法注册回调机制
    ///
    /// TODO: 实现常驻沙盒模式后，可支持真正的跨插件调用
    fn create_call_function<'js>(
        ctx: &Ctx<'js>,
        plugin_id: String,
        _permission_checker: Arc<PermissionChecker>,  // 暂未使用，保留接口
        _call_tx: mpsc::Sender<PluginCallRequest>,    // 暂未使用，保留接口
    ) -> JsResult<Function<'js>> {
        // 调用深度说明：
        // - 当前固定为 1，因为功能未实现，实际不会发生嵌套调用
        // - 未来实现常驻沙盒模式后，需要：
        //   1. 在 PluginContextConfig 中传入当前深度
        //   2. 从事件处理器/方法调用上下文中获取真实深度
        //   3. 每次调用时深度 +1，超过 MAX_CALL_DEPTH 时拒绝
        // - 深度信息仅用于调试输出，不影响功能（返回 not_supported）
        let current_depth = 1usize;

        Function::new(
            ctx.clone(),
            move |_ctx: Ctx<'_>, target: String, method: String, _params: Option<String>| -> JsResult<String> {
                // 跨插件调用功能当前未实现
                //
                // 架构限制说明：
                // 1. 当前插件沙盒是"执行后销毁"模式，不支持持久方法调用
                // 2. rquickjs 0.6 不支持异步 Promise，无法等待远程结果
                // 3. 完整实现需要"常驻沙盒"模式或方法注册回调机制
                //
                // 返回明确的 not_supported 状态，避免调用方误判功能可用
                log::warn!(
                    "[{}] call {}::{} - 功能未实现（需常驻沙盒模式）",
                    plugin_id, target, method
                );

                Ok(serde_json::json!({
                    "success": false,
                    "status": "not_supported",
                    "message": format!(
                        "跨插件调用功能未实现: {}::{}。当前架构不支持方法执行，需要常驻沙盒模式。",
                        target, method
                    ),
                    "target": target,
                    "method": method,
                    // 保留深度信息供调试
                    "call_depth": current_depth + 1,
                    "max_depth": PluginCallRequest::MAX_CALL_DEPTH
                }).to_string())
            },
        )
    }

    /// 创建 log 函数
    ///
    /// `context.log(level, message)` - 带插件前缀的日志
    /// - level: 日志级别 (debug, info, warn, error)
    /// - message: 日志消息
    fn create_log_function<'js>(ctx: &Ctx<'js>, plugin_id: String) -> JsResult<Function<'js>> {
        Function::new(
            ctx.clone(),
            move |_ctx: Ctx<'_>, level: String, message: String| -> JsResult<()> {
                match level.as_str() {
                    "debug" => log::debug!("[plugin:{}] {}", plugin_id, message),
                    "info" => log::info!("[plugin:{}] {}", plugin_id, message),
                    "warn" => log::warn!("[plugin:{}] {}", plugin_id, message),
                    "error" => log::error!("[plugin:{}] {}", plugin_id, message),
                    _ => log::info!("[plugin:{}] {}", plugin_id, message),
                }

                Ok(())
            },
        )
    }

    /// 将 JS Value 转换为 serde_json::Value（简化版本，避免生命周期问题）
    fn value_to_json_simple<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<JsonValue> {
        if value.is_null() || value.is_undefined() {
            return Ok(JsonValue::Null);
        }

        if let Ok(b) = value.get::<bool>() {
            return Ok(JsonValue::Bool(b));
        }

        if let Ok(n) = value.get::<f64>() {
            return Ok(serde_json::Number::from_f64(n)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null));
        }

        if let Ok(s) = value.get::<String>() {
            return Ok(JsonValue::String(s));
        }

        // 尝试 JSON stringify 再 parse
        if let Ok(Some(json_str)) = ctx.json_stringify(value) {
            if let Ok(s) = json_str.to_string() {
                if let Ok(parsed) = serde_json::from_str(&s) {
                    return Ok(parsed);
                }
            }
        }

        Ok(JsonValue::Null)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_call_request() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let request = PluginCallRequest {
            caller: "plugin-a".into(),
            target: "plugin-b".into(),
            method: "getData".into(),
            params: serde_json::json!({"key": "value"}),
            call_depth: 1,
            response_tx: tx,
        };

        assert_eq!(request.caller, "plugin-a");
        assert_eq!(request.target, "plugin-b");
        assert_eq!(request.method, "getData");
        assert_eq!(request.call_depth, 1);
    }

    #[test]
    fn test_emit_request() {
        let request = EmitRequest {
            plugin_id: "test-plugin".into(),
            action: "data_updated".into(),
            data: serde_json::json!({"count": 42}),
        };

        assert_eq!(request.plugin_id, "test-plugin");
        assert_eq!(request.action, "data_updated");
    }
}
