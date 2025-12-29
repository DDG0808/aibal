// PluginError 类实现
// Phase 2.2.9: 实现 PluginError 类注入
//
// 提供给 JS 插件使用的结构化错误类型
// 使用 rquickjs class 宏实现

use rquickjs::{class::Trace, Class, Ctx, Object, Result as JsResult};

/// PluginError - JS 插件错误类型
#[derive(Trace)]
#[rquickjs::class(rename = "PluginError")]
pub struct PluginError {
    #[qjs(skip_trace)]
    error_type: String,
    #[qjs(skip_trace)]
    message: String,
    #[qjs(skip_trace)]
    details: Option<String>,
}

#[rquickjs::methods]
impl PluginError {
    /// 构造函数
    #[qjs(constructor)]
    pub fn new(error_type: String, message: String, details: Option<String>) -> Self {
        Self {
            error_type,
            message,
            details,
        }
    }

    /// type 属性 (只读)
    #[qjs(get, rename = "type")]
    pub fn error_type(&self) -> String {
        self.error_type.clone()
    }

    /// message 属性 (只读)
    #[qjs(get)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// details 属性 (只读)
    #[qjs(get)]
    pub fn details(&self) -> Option<String> {
        self.details.clone()
    }

    /// toString 方法
    #[qjs(rename = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("[{}] {}", self.error_type, self.message)
    }

    /// toJSON 方法
    #[qjs(rename = "toJSON")]
    pub fn to_json(&self) -> String {
        if let Some(ref d) = self.details {
            format!(
                r#"{{"type":"{}","message":"{}","details":{}}}"#,
                self.error_type,
                self.message.replace('"', "\\\""),
                d
            )
        } else {
            format!(
                r#"{{"type":"{}","message":"{}"}}"#,
                self.error_type,
                self.message.replace('"', "\\\"")
            )
        }
    }
}

/// PluginError API 注入器
pub struct PluginErrorApi;

impl PluginErrorApi {
    /// 向上下文注入 PluginError 类
    pub fn inject(ctx: &Ctx<'_>) -> JsResult<()> {
        let globals = ctx.globals();

        // 注册 PluginError 类到全局
        Class::<PluginError>::define(&globals)?;

        // 注入错误类型常量
        let error_types = Object::new(ctx.clone())?;
        error_types.set("NETWORK_ERROR", "NETWORK_ERROR")?;
        error_types.set("AUTH_ERROR", "AUTH_ERROR")?;
        error_types.set("RATE_LIMIT", "RATE_LIMIT")?;
        error_types.set("TIMEOUT", "TIMEOUT")?;
        error_types.set("PARSE_ERROR", "PARSE_ERROR")?;
        error_types.set("PROVIDER_ERROR", "PROVIDER_ERROR")?;
        error_types.set("SANDBOX_LIMIT", "SANDBOX_LIMIT")?;
        error_types.set("PERMISSION_DENIED", "PERMISSION_DENIED")?;
        error_types.set("STORAGE_LIMIT", "STORAGE_LIMIT")?;
        error_types.set("CACHE_ERROR", "CACHE_ERROR")?;
        error_types.set("INCOMPATIBLE_API_VERSION", "INCOMPATIBLE_API_VERSION")?;
        error_types.set("UNKNOWN", "UNKNOWN")?;

        globals.set("PluginErrorType", error_types)?;

        log::debug!("PluginError API 已注入");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_error_type_format() {
        let error_type = "NETWORK_ERROR";
        let message = "Connection failed";
        let result = format!("[{}] {}", error_type, message);
        assert_eq!(result, "[NETWORK_ERROR] Connection failed");
    }
}
