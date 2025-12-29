// Console API 实现
// Phase 2.2.5: 实现 console API
//
// 将 JS console 日志桥接到 Rust tracing/log
// 安全增强：深度限制防止循环引用导致栈溢出

use rquickjs::{prelude::Rest, Ctx, Function, Object, Result as JsResult, Value};

/// 最大字符串化深度（防止循环引用导致栈溢出）
const MAX_STRINGIFY_DEPTH: usize = 10;

/// 单次日志输出最大长度（10KB）
const MAX_STRINGIFY_OUTPUT: usize = 10 * 1024;

/// 数组最大展示元素数量
const MAX_ARRAY_ELEMENTS: usize = 100;

/// Console API
pub struct ConsoleApi;

impl ConsoleApi {
    /// 向上下文注入 console 对象
    pub fn inject(ctx: &Ctx<'_>) -> JsResult<()> {
        let globals = ctx.globals();

        // 创建 console 对象
        let console = Object::new(ctx.clone())?;

        // console.log
        console.set(
            "log",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::info!("[plugin:console] {}", message);
            })?,
        )?;

        // console.info
        console.set(
            "info",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::info!("[plugin:console] {}", message);
            })?,
        )?;

        // console.warn
        console.set(
            "warn",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::warn!("[plugin:console] {}", message);
            })?,
        )?;

        // console.error
        console.set(
            "error",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::error!("[plugin:console] {}", message);
            })?,
        )?;

        // console.debug
        console.set(
            "debug",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::debug!("[plugin:console] {}", message);
            })?,
        )?;

        // console.trace
        console.set(
            "trace",
            Function::new(ctx.clone(), |args: Rest<Value>| {
                let message = format_args(&args.0);
                log::trace!("[plugin:console] {}", message);
            })?,
        )?;

        // 将 console 注入全局对象
        globals.set("console", console)?;

        log::debug!("Console API 已注入");
        Ok(())
    }
}

/// 格式化参数为字符串（带输出截断保护）
fn format_args(args: &[Value]) -> String {
    let result: String = args
        .iter()
        .map(|v| value_to_string_with_depth(v, 0))
        .collect::<Vec<_>>()
        .join(" ");

    // 截断过长输出（使用 UTF-8 安全截断，防止在非边界切片导致 panic）
    if result.len() > MAX_STRINGIFY_OUTPUT {
        // 找到不超过 MAX_STRINGIFY_OUTPUT 的最后一个 UTF-8 字符边界
        let truncate_at = result
            .char_indices()
            .take_while(|(i, _)| *i <= MAX_STRINGIFY_OUTPUT)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        format!(
            "{}... [truncated, {} bytes total]",
            &result[..truncate_at],
            result.len()
        )
    } else {
        result
    }
}

/// 将 JS Value 转换为字符串表示（带深度限制防止循环引用）
fn value_to_string_with_depth(value: &Value, depth: usize) -> String {
    // 深度限制检查
    if depth > MAX_STRINGIFY_DEPTH {
        return "[max depth exceeded]".to_string();
    }

    match value.type_of() {
        rquickjs::Type::Undefined => "undefined".to_string(),
        rquickjs::Type::Null => "null".to_string(),
        rquickjs::Type::Bool => {
            if let Some(b) = value.as_bool() {
                b.to_string()
            } else {
                "boolean".to_string()
            }
        }
        rquickjs::Type::Int => {
            if let Some(n) = value.as_int() {
                n.to_string()
            } else {
                "number".to_string()
            }
        }
        rquickjs::Type::Float => {
            if let Some(n) = value.as_float() {
                n.to_string()
            } else {
                "number".to_string()
            }
        }
        rquickjs::Type::String => {
            if let Some(s) = value.as_string() {
                s.to_string().unwrap_or_else(|_| "string".to_string())
            } else {
                "string".to_string()
            }
        }
        rquickjs::Type::Object => "[object Object]".to_string(),
        rquickjs::Type::Array => {
            if let Some(arr) = value.as_array() {
                // 限制数组元素数量
                let items: Vec<String> = arr
                    .iter()
                    .take(MAX_ARRAY_ELEMENTS)
                    .filter_map(|v| v.ok())
                    .map(|v| value_to_string_with_depth(&v, depth + 1))
                    .collect();

                let arr_len = arr.len();
                if arr_len > MAX_ARRAY_ELEMENTS {
                    format!(
                        "[{}... +{} more]",
                        items.join(", "),
                        arr_len - MAX_ARRAY_ELEMENTS
                    )
                } else {
                    format!("[{}]", items.join(", "))
                }
            } else {
                "[array]".to_string()
            }
        }
        rquickjs::Type::Function => "[function]".to_string(),
        rquickjs::Type::Symbol => "[symbol]".to_string(),
        _ => "[unknown]".to_string(),
    }
}

/// 将 JS Value 转换为字符串表示（公开接口，用于测试兼容）
#[allow(dead_code)]
fn value_to_string(value: &Value) -> String {
    value_to_string_with_depth(value, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_args() {
        let args: Vec<Value> = vec![];
        assert_eq!(format_args(&args), "");
    }
}
