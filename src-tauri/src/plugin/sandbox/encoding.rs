// 编码 API 实现
// Phase 2.2.6-2.2.7: TextEncoder/TextDecoder, atob/btoa
//
// 使用 rquickjs class 宏实现，解决生命周期问题
// TextEncoder/TextDecoder 遵循 WHATWG Encoding 标准
// 安全增强：输入大小限制防止 Rust 堆分配绕过 QuickJS 内存限制

use rquickjs::{
    class::Trace,
    Class, Ctx, Exception, Function, Result as JsResult,
};

/// 最大编码输入大小: 1MB（防止 Rust 堆分配绕过 QuickJS 内存限制）
const MAX_ENCODING_INPUT_SIZE: usize = 1024 * 1024;

/// TextEncoder - 将字符串编码为 UTF-8 字节数组
#[derive(Trace)]
#[rquickjs::class(rename = "TextEncoder")]
pub struct TextEncoder {
    // 空结构体需要一个占位字段
    #[qjs(skip_trace)]
    _private: (),
}

#[rquickjs::methods]
impl TextEncoder {
    /// 构造函数
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// encoding 属性 (只读)
    #[qjs(get)]
    pub fn encoding(&self) -> String {
        "utf-8".to_string()
    }

    /// encode 方法 - 将字符串编码为 Uint8Array（带大小限制）
    pub fn encode(&self, ctx: Ctx<'_>, input: Option<String>) -> JsResult<Vec<u8>> {
        let data = input.unwrap_or_default();
        if data.len() > MAX_ENCODING_INPUT_SIZE {
            return Err(Exception::throw_range(
                &ctx,
                &format!(
                    "Input size {} exceeds maximum of {} bytes",
                    data.len(),
                    MAX_ENCODING_INPUT_SIZE
                ),
            ));
        }
        Ok(data.into_bytes())
    }
}

/// TextDecoder - 将字节数组解码为字符串
#[derive(Trace)]
#[rquickjs::class(rename = "TextDecoder")]
pub struct TextDecoder {
    #[qjs(skip_trace)]
    fatal: bool,
    #[qjs(skip_trace)]
    ignore_bom: bool,
}

#[rquickjs::methods]
impl TextDecoder {
    /// 构造函数
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, label: Option<String>) -> JsResult<Self> {
        let encoding = label.unwrap_or_else(|| "utf-8".to_string()).to_lowercase();

        // 只支持 UTF-8
        if encoding != "utf-8" && encoding != "utf8" {
            return Err(Exception::throw_type(
                &ctx,
                &format!("Unsupported encoding: {}", encoding),
            ));
        }

        Ok(Self {
            fatal: false,
            ignore_bom: false,
        })
    }

    /// encoding 属性 (只读)
    #[qjs(get)]
    pub fn encoding(&self) -> String {
        "utf-8".to_string()
    }

    /// fatal 属性 (只读)
    #[qjs(get)]
    pub fn fatal(&self) -> bool {
        self.fatal
    }

    /// ignoreBOM 属性 (只读)
    #[qjs(get, rename = "ignoreBOM")]
    pub fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    /// decode 方法 - 将字节数组解码为字符串（带大小限制）
    pub fn decode(&self, ctx: Ctx<'_>, input: Option<Vec<u8>>) -> JsResult<String> {
        let bytes = input.unwrap_or_default();
        if bytes.len() > MAX_ENCODING_INPUT_SIZE {
            return Err(Exception::throw_range(
                &ctx,
                &format!(
                    "Input size {} exceeds maximum of {} bytes",
                    bytes.len(),
                    MAX_ENCODING_INPUT_SIZE
                ),
            ));
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// 编码 API 注入器
pub struct EncodingApi;

impl EncodingApi {
    /// 向上下文注入编码相关 API
    pub fn inject(ctx: &Ctx<'_>) -> JsResult<()> {
        let globals = ctx.globals();

        // 注册 TextEncoder 类到全局
        Class::<TextEncoder>::define(&globals)?;

        // 注册 TextDecoder 类到全局
        Class::<TextDecoder>::define(&globals)?;

        // 注入 atob 函数 (Base64 解码，带大小限制)
        globals.set(
            "atob",
            Function::new(ctx.clone(), |ctx: Ctx<'_>, encoded: String| -> JsResult<String> {
                use base64::Engine;

                // 大小限制检查
                if encoded.len() > MAX_ENCODING_INPUT_SIZE {
                    return Err(Exception::throw_range(
                        &ctx,
                        &format!(
                            "Input size {} exceeds maximum of {} bytes",
                            encoded.len(),
                            MAX_ENCODING_INPUT_SIZE
                        ),
                    ));
                }

                match base64::engine::general_purpose::STANDARD.decode(&encoded) {
                    Ok(bytes) => {
                        // atob 返回 Latin1 字符串
                        let result: String = bytes.iter().map(|&b| b as char).collect();
                        Ok(result)
                    }
                    Err(e) => Err(Exception::throw_type(
                        &ctx,
                        &format!("Invalid base64 string: {}", e),
                    )),
                }
            })?,
        )?;

        // 注入 btoa 函数 (Base64 编码，带大小限制)
        globals.set(
            "btoa",
            Function::new(ctx.clone(), |ctx: Ctx<'_>, data: String| -> JsResult<String> {
                use base64::Engine;

                // 大小限制检查
                if data.len() > MAX_ENCODING_INPUT_SIZE {
                    return Err(Exception::throw_range(
                        &ctx,
                        &format!(
                            "Input size {} exceeds maximum of {} bytes",
                            data.len(),
                            MAX_ENCODING_INPUT_SIZE
                        ),
                    ));
                }

                // btoa 只接受 Latin1 范围内的字符
                for c in data.chars() {
                    if c as u32 > 255 {
                        return Err(Exception::throw_type(
                            &ctx,
                            "The string contains characters outside of the Latin1 range",
                        ));
                    }
                }

                let bytes: Vec<u8> = data.chars().map(|c| c as u8).collect();
                Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
            })?,
        )?;

        log::debug!("Encoding API 已注入");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_btoa_atob_roundtrip() {
        let input = "Hello, World!";
        let bytes: Vec<u8> = input.chars().map(|c| c as u8).collect();

        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        let decoded: String = decoded_bytes.iter().map(|&b| b as char).collect();

        assert_eq!(decoded, input);
    }

    #[test]
    fn test_utf8_encoding() {
        let input = "你好世界";
        let bytes = input.as_bytes();
        let decoded = String::from_utf8_lossy(bytes);
        assert_eq!(decoded, input);
    }
}
