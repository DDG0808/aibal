// 安全 Fetch API 实现
// Phase 2.2.1-2.2.4: 安全 fetch API
//
// 提供给 JS 插件使用的安全 HTTP 请求功能
// 包含 URL 模式拦截、DNS 级别拦截、响应大小限制

use std::convert::TryFrom;
use std::fmt;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use std::collections::HashMap;

use futures::StreamExt;
use rquickjs::{
    class::Trace, function::Opt, Class, Ctx, FromJs, Function, IntoJs, Object,
    Result as JsResult, Value,
};

// ============================================================================
// Fetch Options 结构体
// ============================================================================

/// Fetch 请求选项
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// HTTP 方法（GET, POST, PUT, DELETE 等）
    pub method: Option<String>,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体
    pub body: Option<String>,
}

impl<'js> FromJs<'js> for FetchOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if value.is_undefined() || value.is_null() {
            return Ok(Self::default());
        }

        let obj = Object::from_js(ctx, value)?;

        // 解析 method
        let method: Option<String> = obj.get("method").ok();

        // 解析 headers
        let mut headers = HashMap::new();
        if let Ok(headers_val) = obj.get::<_, Value>("headers") {
            if !headers_val.is_undefined() && !headers_val.is_null() {
                if let Ok(headers_obj) = Object::from_value(headers_val) {
                    for prop in headers_obj.props::<String, String>() {
                        if let Ok((key, val)) = prop {
                            headers.insert(key, val);
                        }
                    }
                }
            }
        }

        // 解析 body
        let body: Option<String> = obj.get("body").ok();

        Ok(Self {
            method,
            headers,
            body,
        })
    }
}

// ============================================================================
// 统一错误类型
// ============================================================================

/// Fetch API 统一错误类型
#[derive(Debug, Clone)]
pub enum FetchError {
    /// URL 无效或不安全
    InvalidUrl(String),
    /// DNS 解析失败或检测到 rebinding 攻击
    DnsError(String),
    /// 并发请求数超限
    TooManyRequests,
    /// 响应体过大
    ResponseTooLarge { size: usize, max: usize },
    /// Content-Length 超出平台限制
    ContentLengthOverflow(u64),
    /// 网络请求失败
    NetworkError(String),
    /// 响应读取失败
    ReadError(String),
    /// HTTP 客户端未初始化（创建失败）
    ClientNotInitialized,
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            Self::DnsError(msg) => write!(f, "DNS error: {}", msg),
            Self::TooManyRequests => write!(f, "Too many concurrent requests"),
            Self::ResponseTooLarge { size, max } => {
                write!(f, "Response size {} exceeds maximum of {} bytes", size, max)
            }
            Self::ContentLengthOverflow(len) => {
                write!(f, "Content-Length {} exceeds platform maximum", len)
            }
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ReadError(msg) => write!(f, "Read error: {}", msg),
            Self::ClientNotInitialized => write!(f, "HTTP client not initialized (creation failed)"),
        }
    }
}

impl std::error::Error for FetchError {}

/// 最大响应大小: 10MB
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// 默认请求超时: 30 秒
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// 最大并发请求数
const MAX_CONCURRENT_REQUESTS: usize = 10;

/// DNS 解析超时: 5 秒
const DNS_TIMEOUT: Duration = Duration::from_secs(5);

/// FetchResult - fetch 请求结果
#[derive(Trace)]
#[rquickjs::class(rename = "FetchResult")]
pub struct FetchResult {
    #[qjs(skip_trace)]
    url: String,
    #[qjs(skip_trace)]
    method: String,
    #[qjs(skip_trace)]
    ok: bool,
    #[qjs(skip_trace)]
    status: u16,
    #[qjs(skip_trace)]
    body: String,
}

#[rquickjs::methods]
impl FetchResult {
    /// 构造函数
    #[qjs(constructor)]
    pub fn new(url: String, method: String, ok: bool, status: u16, body: String) -> Self {
        Self {
            url,
            method,
            ok,
            status,
            body,
        }
    }

    /// url 属性
    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// method 属性
    #[qjs(get)]
    pub fn method(&self) -> String {
        self.method.clone()
    }

    /// ok 属性
    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.ok
    }

    /// status 属性
    #[qjs(get)]
    pub fn status(&self) -> u16 {
        self.status
    }

    /// text 方法 - 返回响应体文本
    pub fn text(&self) -> String {
        self.body.clone()
    }

    /// json 方法 - 返回响应体（假设是 JSON）
    pub fn json(&self) -> String {
        self.body.clone()
    }
}

// ============================================================================
// 异步返回的数据结构（用于 Async<T> Promise 转换）
// ============================================================================

/// 用于异步 fetch 返回的数据结构
/// 与 FetchResult 类分离，因为 Async 需要 'static 生命周期
/// 实现 IntoJs 以便在 Promise resolve 时自动转换为 JS 对象
#[derive(Debug, Clone)]
struct FetchResultData {
    url: String,
    method: String,
    ok: bool,
    status: u16,
    body: String,
}

impl<'js> IntoJs<'js> for FetchResultData {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let obj = Object::new(ctx.clone())?;

        // 设置基础属性
        obj.set("url", self.url.clone())?;
        obj.set("method", self.method.clone())?;
        obj.set("ok", self.ok)?;
        obj.set("status", self.status)?;
        obj.set("_body", self.body.clone())?;

        // text() 方法
        let body_for_text = self.body.clone();
        obj.set(
            "text",
            Function::new(ctx.clone(), move || -> JsResult<String> {
                Ok(body_for_text.clone())
            })?,
        )?;

        // json() 方法 - 使用 serde_json 解析
        obj.set(
            "json",
            Function::new(
                ctx.clone(),
                |ctx: Ctx<'js>, this: rquickjs::function::This<Object<'js>>| -> JsResult<Value<'js>> {
                    let body: String = this.0.get("_body")?;
                    let clean_body = body
                        .trim_start_matches('\u{FEFF}')
                        .trim();

                    let json_value: serde_json::Value = serde_json::from_str(clean_body)
                        .map_err(|e| rquickjs::Error::new_from_js_message("json", "object", &format!("JSON parse error: {}", e)))?;

                    serde_json_to_js(&ctx, &json_value)
                },
            )?,
        )?;

        Ok(obj.into_value())
    }
}

/// 将 serde_json::Value 转换为 rquickjs::Value
fn serde_json_to_js<'js>(ctx: &Ctx<'js>, value: &serde_json::Value) -> JsResult<Value<'js>> {
    match value {
        serde_json::Value::Null => Ok(Value::new_null(ctx.clone())),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_float(ctx.clone(), 0.0))
            }
        }
        serde_json::Value::String(s) => {
            rquickjs::String::from_str(ctx.clone(), s)
                .map(|s| s.into_value())
        }
        serde_json::Value::Array(arr) => {
            let js_arr = rquickjs::Array::new(ctx.clone())?;
            for (i, item) in arr.iter().enumerate() {
                let js_val = serde_json_to_js(ctx, item)?;
                js_arr.set(i, js_val)?;
            }
            Ok(js_arr.into_value())
        }
        serde_json::Value::Object(map) => {
            let js_obj = Object::new(ctx.clone())?;
            for (k, v) in map {
                let js_val = serde_json_to_js(ctx, v)?;
                js_obj.set(k.as_str(), js_val)?;
            }
            Ok(js_obj.into_value())
        }
    }
}

/// Fetch API
pub struct FetchApi;

impl FetchApi {
    /// 向上下文注入异步 fetch 函数
    ///
    /// 使用 rquickjs 的 Async<T> 包装器实现真正的异步 fetch
    /// JS 端调用 fetch(url) 返回 Promise，resolve 为包含响应数据的对象
    ///
    /// # 安全特性
    /// 调用 secure_fetch，包含完整的安全检查：
    /// - URL 模式检查（禁止 localhost、私有 IP、内部域名）
    /// - DNS rebinding 防护（resolve API 固定 IP）
    /// - 并发限制（RAII Guard）
    /// - 响应大小限制（流式读取）
    ///
    /// # 实现说明
    /// 使用 `Async<Fn>` 包装器：
    /// - 闭包返回 Future，rquickjs 自动转换为 JS Promise
    /// - Future 完成时 Promise resolve/reject
    /// - 需要 runtime 正确处理 pending jobs（调用 idle()）
    pub fn inject(ctx: &Ctx<'_>, manager: Arc<RequestManager>) -> JsResult<()> {
        let globals = ctx.globals();

        // 注册 FetchResult 类（保留用于类型文档和向后兼容）
        Class::<FetchResult>::define(&globals)?;

        // 克隆 manager 用于闭包捕获
        let manager_for_fetch = manager.clone();

        // 注入同步 fetch 函数
        // 使用新线程 + channel 阻塞执行，避免 Promise 链问题
        globals.set(
            "fetch",
            Function::new(
                ctx.clone(),
                move |url: String, options: Opt<FetchOptions>| {
                    // 克隆 Arc 用于请求
                    let manager = manager_for_fetch.clone();
                    let url_owned = url;
                    let opts = options.0.unwrap_or_default();

                    // 1. URL 安全检查（同步）
                    if let Err(e) = UrlSecurityChecker::check_url(&url_owned) {
                        log::warn!("Fetch API URL 检查失败: {} -> {}", url_owned, e);
                        return FetchResultData {
                            url: url_owned,
                            method: opts.method.clone().unwrap_or_else(|| "GET".to_string()),
                            ok: false,
                            status: 0,
                            body: format!("URL validation failed: {}", e),
                        };
                    }

                    let method = opts.method.clone().unwrap_or_else(|| "GET".to_string());
                    log::debug!("Fetch API 开始同步请求: {} {}", method, url_owned);

                    // 2. 使用新线程 + channel 同步执行异步 fetch
                    let (tx, rx) = std::sync::mpsc::channel();
                    let url_for_thread = url_owned.clone();
                    let opts_for_thread = opts.clone();

                    std::thread::spawn(move || {
                        // 在新线程中创建 tokio runtime 执行异步请求
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("Failed to create tokio runtime");

                        let result = rt.block_on(async {
                            Self::secure_fetch_with_options(&manager, &url_for_thread, &opts_for_thread).await
                        });

                        let _ = tx.send(result);
                    });

                    // 等待结果（30 秒超时）
                    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                        Ok(Ok(result)) => {
                            log::debug!(
                                "Fetch API 请求成功: {} -> status {}",
                                url_owned,
                                result.status
                            );
                            FetchResultData {
                                url: result.url,
                                method: result.method,
                                ok: result.ok,
                                status: result.status,
                                body: result.body,
                            }
                        }
                        Ok(Err(e)) => {
                            log::warn!("Fetch API 请求失败: {} -> {}", url_owned, e);
                            FetchResultData {
                                url: url_owned,
                                method,
                                ok: false,
                                status: 0,
                                body: format!("Fetch error: {}", e),
                            }
                        }
                        Err(_) => {
                            log::warn!("Fetch API 请求超时: {}", url_owned);
                            FetchResultData {
                                url: url_owned,
                                method,
                                ok: false,
                                status: 0,
                                body: "Fetch timeout".to_string(),
                            }
                        }
                    }
                },
            )?,
        )?;

        log::debug!("Fetch API 已注入（同步阻塞模式）");
        Ok(())
    }

    /// 安全的异步 fetch 实现（带 DNS rebinding 防护）
    /// 这是供 Rust 层使用的安全 API，JS 层的 fetch 最终应调用此方法
    ///
    /// 安全特性：
    /// - URL 模式检查：禁止访问 localhost、私有 IP、内部域名
    /// - DNS rebinding 防护：解析后检查 IP，并使用 resolve API 固定 IP
    /// - 禁用 redirect：防止通过 redirect 绕过 DNS 检查
    /// - 禁用 proxy：防止通过环境变量绕过 SSRF 检查
    /// - RAII 守卫：异步取消时也能正确释放并发槽位
    /// - 流式大小限制：分块读取，超限立即中断
    ///
    /// # DNS TOCTOU 防护
    /// 本实现通过 reqwest 的 `resolve()` API 消除了 TOCTOU 窗口：
    /// 1. 步骤 2 进行 DNS 解析并检查所有 IP 是否为私网地址
    /// 2. 步骤 4 使用 `resolve()` API 强制 reqwest 使用步骤 2 验证过的 IP
    /// 这样 reqwest 不会重新解析 DNS，攻击者无法在检查后修改 DNS 记录。
    ///
    /// 额外防护层：
    /// - 禁用 redirect：即使某处绕过，也无法通过 redirect 跳转到内网
    /// - 禁用 proxy：防止通过代理访问内网
    /// - 空 DNS 结果返回错误：不允许任何退化路径
    pub async fn secure_fetch(
        manager: &RequestManager,
        url_str: &str,
    ) -> Result<FetchResult, FetchError> {
        // 1. URL 模式检查（同步，快速失败）
        let parsed_url = UrlSecurityChecker::check_url(url_str)?;

        // 2. 使用 RAII 守卫获取请求槽位（在 DNS 解析前！）
        // 这样可以限制 DNS 阶段的并发数，防止 DNS DoS 攻击
        // Guard 在作用域结束或异步取消时自动释放槽位
        let _guard = RequestGuard::acquire(manager)?;

        // 3. DNS 解析后检查（防止 DNS rebinding 攻击）
        // 返回预解析的 IP 用于后续 resolve API，消除 TOCTOU 窗口
        // 注意：此阶段也受并发限制保护
        let resolved_ip = UrlSecurityChecker::check_resolved_ip(&parsed_url).await?;

        // 4. 执行实际的 fetch 请求（使用预解析的 IP，消除 TOCTOU）
        // 无论成功失败，_guard 的 Drop 都会释放槽位
        let (ok, status, body) = Self::do_fetch_with_resolved_ip(
            manager,
            &parsed_url,
            resolved_ip,
        ).await?;

        Ok(FetchResult::new(
            url_str.to_string(),
            "GET".to_string(),
            ok,
            status,
            body,
        ))
    }

    /// 带 options 的安全 fetch 实现
    ///
    /// 支持自定义 HTTP 方法、请求头和请求体
    pub async fn secure_fetch_with_options(
        manager: &RequestManager,
        url_str: &str,
        options: &FetchOptions,
    ) -> Result<FetchResult, FetchError> {
        // 1. URL 模式检查（同步，快速失败）
        let parsed_url = UrlSecurityChecker::check_url(url_str)?;

        // 2. 使用 RAII 守卫获取请求槽位
        let _guard = RequestGuard::acquire(manager)?;

        // 3. DNS 解析后检查
        let resolved_ip = UrlSecurityChecker::check_resolved_ip(&parsed_url).await?;

        // 4. 执行实际的 fetch 请求
        let method = options.method.clone().unwrap_or_else(|| "GET".to_string());
        let (ok, status, body) = Self::do_fetch_with_options(
            &parsed_url,
            resolved_ip,
            options,
            manager.max_response_size(),
        )
        .await?;

        Ok(FetchResult::new(url_str.to_string(), method, ok, status, body))
    }

    /// 使用 options 和预解析 IP 的 fetch 实现
    async fn do_fetch_with_options(
        parsed_url: &url::Url,
        resolved_addr: Option<std::net::SocketAddr>,
        options: &FetchOptions,
        max_size: usize,
    ) -> Result<(bool, u16, String), FetchError> {
        let host = parsed_url.host_str().unwrap_or_default();

        let addr = resolved_addr
            .ok_or_else(|| FetchError::DnsError("No resolved IP address available".to_string()))?;

        // 创建 Client
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve(host, addr)
            .build()
            .map_err(|e| FetchError::NetworkError(format!("Failed to create client: {}", e)))?;

        // 构建请求
        let method_str = options.method.as_deref().unwrap_or("GET").to_uppercase();
        let method = reqwest::Method::from_bytes(method_str.as_bytes())
            .unwrap_or(reqwest::Method::GET);

        let mut request = client.request(method, parsed_url.as_str());

        // 添加请求头
        for (key, value) in &options.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // 添加请求体
        if let Some(body) = &options.body {
            request = request.body(body.clone());
        }

        // 发送请求
        let response = request
            .send()
            .await
            .map_err(|e| FetchError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();
        let ok = response.status().is_success();

        // 检查 Content-Length
        if let Some(content_length) = response.content_length() {
            let len_usize = usize::try_from(content_length)
                .map_err(|_| FetchError::ContentLengthOverflow(content_length))?;

            if len_usize > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: len_usize,
                    max: max_size,
                });
            }
        }

        // 流式读取
        let mut stream = response.bytes_stream();
        let mut body_bytes = Vec::with_capacity(max_size.min(64 * 1024));
        let mut total_size: usize = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| FetchError::ReadError(e.to_string()))?;

            total_size = total_size.checked_add(chunk.len()).ok_or_else(|| {
                FetchError::ResponseTooLarge {
                    size: usize::MAX,
                    max: max_size,
                }
            })?;

            if total_size > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: total_size,
                    max: max_size,
                });
            }

            body_bytes.extend_from_slice(&chunk);
        }

        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        Ok((ok, status, body))
    }

    /// 使用预解析 IP 的 fetch 实现，消除 DNS TOCTOU 窗口
    ///
    /// 通过 reqwest 的 resolve API 强制使用预先验证过的 IP 地址，
    /// 避免 reqwest 内部重新解析 DNS 导致的 TOCTOU 漏洞。
    async fn do_fetch_with_resolved_ip(
        manager: &RequestManager,
        parsed_url: &url::Url,
        resolved_addr: Option<std::net::SocketAddr>,
    ) -> Result<(bool, u16, String), FetchError> {
        let host = parsed_url.host_str().unwrap_or_default();
        let max_size = manager.max_response_size();

        // 必须有预解析的 IP，否则返回错误
        // 这是安全关键路径，不允许退化到可能重新解析 DNS 的默认 client
        let addr = resolved_addr.ok_or_else(|| {
            FetchError::DnsError("No resolved IP address available".to_string())
        })?;

        // 创建使用 resolve API 的临时 Client
        // 这确保 reqwest 使用我们验证过的 IP，而不是重新解析 DNS
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve(host, addr)
            .build()
            .map_err(|e| FetchError::NetworkError(format!("Failed to create client: {}", e)))?;

        let response = client
            .get(parsed_url.as_str())
            .send()
            .await
            .map_err(|e| FetchError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();
        let ok = response.status().is_success();

        // 检查 Content-Length
        if let Some(content_length) = response.content_length() {
            let len_usize = usize::try_from(content_length)
                .map_err(|_| FetchError::ContentLengthOverflow(content_length))?;

            if len_usize > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: len_usize,
                    max: max_size,
                });
            }
        }

        // 流式读取
        let mut stream = response.bytes_stream();
        let mut body_bytes = Vec::with_capacity(max_size.min(64 * 1024));
        let mut total_size: usize = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| FetchError::ReadError(e.to_string()))?;

            total_size = total_size.checked_add(chunk.len()).ok_or_else(|| {
                FetchError::ResponseTooLarge {
                    size: usize::MAX,
                    max: max_size,
                }
            })?;

            if total_size > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: total_size,
                    max: max_size,
                });
            }

            body_bytes.extend_from_slice(&chunk);
        }

        let body = String::from_utf8_lossy(&body_bytes).into_owned();
        Ok((ok, status, body))
    }

    /// 内部 fetch 实现（不使用预解析 IP，已弃用）
    #[allow(dead_code)]
    async fn do_fetch(
        manager: &RequestManager,
        parsed_url: &url::Url,
    ) -> Result<(bool, u16, String), FetchError> {
        // 获取响应头（带大小检查）
        let response = manager
            .client()?
            .get(parsed_url.as_str())
            .send()
            .await
            .map_err(|e| FetchError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();
        let ok = response.status().is_success();
        let max_size = manager.max_response_size();

        // 检查 Content-Length（如果存在）以便提前拒绝过大响应
        // 使用 TryFrom 进行类型安全转换，避免 32 位系统上的截断问题
        if let Some(content_length) = response.content_length() {
            let len_usize = usize::try_from(content_length)
                .map_err(|_| FetchError::ContentLengthOverflow(content_length))?;

            if len_usize > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: len_usize,
                    max: max_size,
                });
            }
        }

        // 使用流式读取强制大小限制
        // 每个 chunk 都检查累计大小，超限立即中断
        // 这样即使恶意服务器发送超大 chunked 响应也能及时终止
        let mut stream = response.bytes_stream();
        let mut body_bytes = Vec::with_capacity(max_size.min(64 * 1024)); // 初始预分配 64KB
        let mut total_size: usize = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk =
                chunk_result.map_err(|e| FetchError::ReadError(e.to_string()))?;

            // 使用 checked_add 防止溢出
            total_size = total_size.checked_add(chunk.len()).ok_or_else(|| {
                FetchError::ResponseTooLarge {
                    size: usize::MAX,
                    max: max_size,
                }
            })?;

            // 超限立即终止，不继续接收数据
            if total_size > max_size {
                return Err(FetchError::ResponseTooLarge {
                    size: total_size,
                    max: max_size,
                });
            }

            body_bytes.extend_from_slice(&chunk);
        }

        let body = String::from_utf8_lossy(&body_bytes).into_owned();

        Ok((ok, status, body))
    }
}

// ============================================================================
// Request Manager
// ============================================================================

/// 请求管理器
/// 使用 AtomicUsize 进行无锁并发控制，避免 RAII Guard 的 blocking_lock 问题
pub struct RequestManager {
    /// HTTP 客户端（禁用 redirect 防止 DNS rebinding 绕过）
    /// Option 用于处理极端情况下客户端创建失败
    client: Option<reqwest::Client>,
    /// 活跃请求数（原子计数器，无锁操作）
    active_requests: AtomicUsize,
}

impl RequestManager {
    /// 创建新的请求管理器
    /// 返回 Result 以避免 panic=abort 时进程崩溃
    ///
    /// 安全配置：
    /// - 禁用 redirect：防止恶意服务器通过 redirect 绕过 DNS rebinding 检查
    /// - 禁用 proxy：防止通过 HTTP_PROXY 环境变量绕过 SSRF 检查
    /// - 设置超时：防止请求无限挂起
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("CUK-Plugin/1.0")
            // 禁用 redirect，防止 DNS rebinding 绕过
            // 恶意服务器可能先返回公网 IP，然后 redirect 到内网 IP
            .redirect(reqwest::redirect::Policy::none())
            // 禁用 proxy，防止通过 HTTP_PROXY/HTTPS_PROXY 环境变量绕过 SSRF 检查
            // 攻击者可能设置 proxy 指向本地，从而访问内网资源
            .no_proxy()
            .build()?;

        Ok(Self {
            client: Some(client),
            active_requests: AtomicUsize::new(0),
        })
    }

    /// 创建请求管理器，失败时使用最小化安全配置
    ///
    /// 安全保证：
    /// - 优先使用完整配置
    /// - fallback 仍保持安全配置（禁用 redirect、proxy，保留 timeout）
    /// - **不会 panic**：双重失败时返回禁用状态，所有请求返回错误
    /// - 不会降级到不安全的默认客户端
    ///
    /// # 双重失败处理（关键修复）
    /// 如果主配置和 fallback 都失败，不再 panic，而是：
    /// 1. 记录 CRITICAL 级别日志
    /// 2. 返回 client=None 的 RequestManager
    /// 3. 所有后续请求返回 `FetchError::ClientNotInitialized`
    pub fn new_with_fallback() -> Self {
        Self::new().unwrap_or_else(|e| {
            log::warn!("Failed to create custom HTTP client: {}, trying minimal fallback", e);

            // 唯一的 fallback：最小安全配置
            // 必须保持安全性，即使牺牲功能
            match reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .connect_timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build()
            {
                Ok(client) => {
                    log::info!("Using minimal secure HTTP client (fallback mode)");
                    Self {
                        client: Some(client),
                        active_requests: AtomicUsize::new(0),
                    }
                }
                Err(e2) => {
                    // 极端情况：无法创建任何客户端
                    // 关键修复：不再 panic，而是记录错误并返回禁用状态
                    log::error!(
                        "CRITICAL: Cannot create any HTTP client. Primary error: {}, Fallback error: {}. \
                        Fetch API will be disabled for this session.",
                        e, e2
                    );
                    Self {
                        client: None,
                        active_requests: AtomicUsize::new(0),
                    }
                }
            }
        })
    }

    /// 检查客户端是否可用
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }

    /// 获取客户端（返回 Result）
    ///
    /// 如果客户端未初始化（双重创建失败），返回错误
    pub fn client(&self) -> Result<&reqwest::Client, FetchError> {
        self.client.as_ref().ok_or(FetchError::ClientNotInitialized)
    }

    /// 获取客户端（不安全版本，用于内部操作）
    ///
    /// # Panics
    /// 如果客户端未初始化会 panic，仅用于调用方已检查 is_available 的情况
    #[allow(dead_code)]
    fn client_unchecked(&self) -> &reqwest::Client {
        self.client.as_ref().expect("HTTP client not initialized")
    }

    /// 获取最大响应大小
    pub fn max_response_size(&self) -> usize {
        MAX_RESPONSE_SIZE
    }

    /// 开始请求（原子操作，无锁）
    /// 使用 CAS 循环确保并发安全
    pub fn start_request(&self) -> Result<(), FetchError> {
        loop {
            let current = self.active_requests.load(Ordering::Acquire);
            if current >= MAX_CONCURRENT_REQUESTS {
                return Err(FetchError::TooManyRequests);
            }
            // CAS：如果当前值仍是 current，则加 1
            if self
                .active_requests
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(());
            }
            // CAS 失败，重试
        }
    }

    /// 结束请求（原子操作，无锁，无瞬时下溢）
    ///
    /// 使用 fetch_update 确保原子性：
    /// - 只有当 count > 0 时才减 1
    /// - 避免瞬时下溢（值变为 usize::MAX）导致其他线程误判
    pub fn end_request(&self) {
        let result = self.active_requests.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| {
                if current > 0 {
                    Some(current - 1)
                } else {
                    None // 不更新，返回错误
                }
            },
        );

        if result.is_err() {
            log::warn!("RequestManager: end_request called when count was 0 (no-op)");
        }
    }

    /// 重置请求计数（仅用于测试或插件卸载）
    ///
    /// # 安全警告
    /// 此方法会破坏并发控制的一致性：
    /// - 已存在的 RequestGuard 在 Drop 时会调用 end_request
    /// - 如果在有活跃 Guard 时调用此方法，后续 end_request 可能导致计数不一致
    ///
    /// 仅在以下场景安全调用：
    /// 1. 确保没有活跃的 RequestGuard
    /// 2. 插件完全卸载时（所有请求已终止）
    /// 3. 测试环境
    #[cfg(any(test, debug_assertions))]
    pub fn reset_for_testing(&self) {
        let old = self.active_requests.swap(0, Ordering::AcqRel);
        if old > 0 {
            log::warn!(
                "RequestManager: reset_for_testing called with {} active requests",
                old
            );
        }
        log::debug!("已重置请求计数（仅限测试）");
    }

    /// 获取当前待处理的请求数（只读查询）
    ///
    /// 此方法仅返回活跃请求计数，不执行任何取消操作。
    /// 实际取消请求需要通过取消相应的 tokio 任务实现。
    ///
    /// # 用途
    /// - 监控：检查是否有活跃请求
    /// - 优雅关闭：等待计数归零后再关闭
    /// - 调试：诊断请求泄漏
    ///
    /// # 注意
    /// - 返回的是调用时刻的快照，可能立即过时
    /// - 如需等待所有请求完成，应结合轮询使用
    #[inline]
    pub fn pending_count(&self) -> usize {
        let count = self.active_requests.load(Ordering::Acquire);
        if count > 0 {
            log::debug!("RequestManager: {} 个请求待处理", count);
        }
        count
    }

    /// 获取活跃请求数（pending_count 的别名）
    #[inline]
    pub fn active_count(&self) -> usize {
        self.active_requests.load(Ordering::Acquire)
    }
}

impl Default for RequestManager {
    fn default() -> Self {
        Self::new_with_fallback()
    }
}

// ============================================================================
// RAII Request Guard（异步取消安全）
// ============================================================================

/// 请求槽位守卫，确保在异步取消场景下也能正确释放槽位
/// 使用 RAII 模式：创建时获取槽位，Drop 时自动释放
///
/// 关键设计：
/// - 使用 AtomicUsize 进行无锁并发控制
/// - Drop 是同步的原子操作，不会阻塞 Tokio 工作线程
/// - 即使在异步取消（超时/drop）场景下也能正确释放
pub struct RequestGuard<'a> {
    manager: &'a RequestManager,
    /// 标记是否已释放（避免重复释放）
    released: bool,
}

impl<'a> RequestGuard<'a> {
    /// 尝试获取请求槽位，成功返回 Guard
    /// 注意：这是同步操作（原子 CAS），不需要 async
    pub fn acquire(manager: &'a RequestManager) -> Result<Self, FetchError> {
        manager.start_request()?;
        Ok(Self {
            manager,
            released: false,
        })
    }

    /// 手动释放槽位（可选，Drop 时也会自动释放）
    pub fn release(&mut self) {
        if !self.released {
            self.manager.end_request();
            self.released = true;
        }
    }
}

impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        // 原子操作，无锁，不会阻塞 Tokio 工作线程
        if !self.released {
            self.manager.end_request();
            self.released = true;
        }
    }
}

// ============================================================================
// URL 安全检查
// ============================================================================

/// URL 安全检查器
pub struct UrlSecurityChecker;

impl UrlSecurityChecker {
    /// 检查 URL 是否安全
    pub fn check_url(url_str: &str) -> Result<url::Url, FetchError> {
        // 解析 URL
        let parsed = url::Url::parse(url_str)
            .map_err(|e| FetchError::InvalidUrl(format!("Parse error: {}", e)))?;

        // 只允许 HTTP 和 HTTPS
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(FetchError::InvalidUrl(format!(
                    "Unsupported scheme: {}",
                    scheme
                )))
            }
        }

        // 检查主机名
        let host = parsed
            .host_str()
            .ok_or_else(|| FetchError::InvalidUrl("URL must have a host".to_string()))?;

        // 禁止 localhost（字符串匹配）
        if host == "localhost" || host == "127.0.0.1" {
            return Err(FetchError::InvalidUrl(
                "Access to localhost is forbidden".to_string(),
            ));
        }

        // 使用 parsed.host() 获取结构化的 Host 信息，正确处理 IPv6
        match parsed.host() {
            Some(url::Host::Ipv4(ipv4)) => {
                if Self::is_private_ip(&IpAddr::V4(ipv4)) {
                    return Err(FetchError::InvalidUrl(
                        "Access to private IP addresses is forbidden".to_string(),
                    ));
                }
            }
            Some(url::Host::Ipv6(ipv6)) => {
                if Self::is_private_ip(&IpAddr::V6(ipv6)) {
                    return Err(FetchError::InvalidUrl(
                        "Access to private IP addresses is forbidden".to_string(),
                    ));
                }
            }
            Some(url::Host::Domain(_)) => {
                // 域名在下面的私有 IP 检查中会尝试解析
                // 这里先跳过，使用 host_str 进行字符串匹配
            }
            None => {
                return Err(FetchError::InvalidUrl(
                    "URL must have a host".to_string(),
                ));
            }
        }

        // 对于域名，仍然尝试解析为 IP（处理像 "127.0.0.1" 这样的字符串情况）
        if let Ok(ip) = host.parse::<IpAddr>() {
            if Self::is_private_ip(&ip) {
                return Err(FetchError::InvalidUrl(
                    "Access to private IP addresses is forbidden".to_string(),
                ));
            }
        }

        // 禁止内部域名模式
        let lower_host = host.to_lowercase();
        if lower_host.ends_with(".local")
            || lower_host.ends_with(".internal")
            || lower_host.ends_with(".localhost")
            || lower_host.contains("169.254.")
        {
            return Err(FetchError::InvalidUrl(
                "Access to internal domains is forbidden".to_string(),
            ));
        }

        Ok(parsed)
    }

    /// 检查 IP 是否为非公网地址（完整的 IPv4/IPv6 私网/特殊地址检测）
    ///
    /// 注意：此函数等效于 `!IpAddr::is_global()`，但 `is_global()` 目前仅在 nightly Rust 中可用。
    /// 当 `is_global()` 稳定后，应替换为 `!ip.is_global()` 以获得更完整的检测。
    ///
    /// 检测范围：
    /// - 私有地址（RFC 1918）
    /// - 环回地址
    /// - 链路本地地址
    /// - 广播/未指定地址
    /// - 文档用途地址
    /// - CGNAT 地址
    /// - 保留地址
    /// - 多播地址
    fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                ipv4.is_private()           // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                    || ipv4.is_loopback()   // 127.0.0.0/8
                    || ipv4.is_link_local() // 169.254.0.0/16
                    || ipv4.is_broadcast()  // 255.255.255.255
                    || ipv4.is_unspecified() // 0.0.0.0
                    || ipv4.is_documentation() // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
                    || ipv4.is_multicast()  // 224.0.0.0/4
                    || octets[0] == 100 && (octets[1] & 0xC0) == 64 // CGNAT 100.64.0.0/10
                    || octets[0] == 0       // 0.0.0.0/8（当前网络）
                    || octets[0] >= 240     // 240.0.0.0/4（保留）和 255.255.255.255
                    || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0) // 192.0.0.0/24（IANA 保留）
                    || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)) // 198.18.0.0/15（基准测试）
            }
            IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                ipv6.is_loopback()      // ::1
                    || ipv6.is_unspecified()  // ::
                    || ipv6.is_multicast() // ff00::/8
                    // ULA (fc00::/7) - Unique Local Address
                    || (segments[0] & 0xfe00) == 0xfc00
                    // Link-local (fe80::/10)
                    || (segments[0] & 0xffc0) == 0xfe80
                    // Deprecated site-local (fec0::/10)
                    || (segments[0] & 0xffc0) == 0xfec0
                    // IPv4-mapped addresses (::ffff:0:0/96) - 检查映射的 IPv4 是否私有
                    || (segments[0] == 0 && segments[1] == 0 && segments[2] == 0
                        && segments[3] == 0 && segments[4] == 0 && segments[5] == 0xffff
                        && Self::is_private_ip(&IpAddr::V4(std::net::Ipv4Addr::new(
                            (segments[6] >> 8) as u8,
                            (segments[6] & 0xff) as u8,
                            (segments[7] >> 8) as u8,
                            (segments[7] & 0xff) as u8,
                        ))))
                    // IPv4-compatible addresses (deprecated, ::x.x.x.x)
                    || (segments[0] == 0 && segments[1] == 0 && segments[2] == 0
                        && segments[3] == 0 && segments[4] == 0 && segments[5] == 0
                        && (segments[6] != 0 || segments[7] != 0) // 不是 ::
                        && (segments[6] != 0 || segments[7] != 1)) // 不是 ::1
                    // Teredo (2001:0000::/32) - 可能隧道到私有 IPv4
                    || (segments[0] == 0x2001 && segments[1] == 0)
                    // Documentation (2001:db8::/32)
                    || (segments[0] == 0x2001 && segments[1] == 0x0db8)
                    // Discard-only (100::/64)
                    || (segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0)
            }
        }
    }

    /// DNS 解析后检查 IP，返回第一个安全的 IP 地址
    ///
    /// 返回值用于后续的 resolve API，消除 TOCTOU 窗口
    pub async fn check_resolved_ip(url: &url::Url) -> Result<Option<std::net::SocketAddr>, FetchError> {
        let host = url
            .host_str()
            .ok_or_else(|| FetchError::InvalidUrl("No host".to_string()))?;

        // 如果是 IP 地址，已经在 check_url 中检查过了
        if let Ok(ip) = host.parse::<IpAddr>() {
            let port = url.port_or_known_default().unwrap_or(80);
            return Ok(Some(std::net::SocketAddr::new(ip, port)));
        }

        // DNS 解析（带超时保护，防止 DoS）
        let lookup_addr = format!("{}:{}", host, url.port_or_known_default().unwrap_or(80));
        let addrs: Vec<_> = tokio::time::timeout(
            DNS_TIMEOUT,
            tokio::net::lookup_host(&lookup_addr),
        )
        .await
        .map_err(|_| FetchError::DnsError(format!(
            "DNS resolution timeout after {:?} for {}",
            DNS_TIMEOUT, host
        )))?
        .map_err(|e| FetchError::DnsError(format!("DNS resolution failed: {}", e)))?
        .collect();

        // 检查所有解析结果，找到第一个安全的 IP
        if addrs.is_empty() {
            return Err(FetchError::DnsError(format!(
                "DNS resolution returned no addresses for {}",
                host
            )));
        }

        let mut safe_addr = None;
        for addr in &addrs {
            if Self::is_private_ip(&addr.ip()) {
                return Err(FetchError::DnsError(format!(
                    "DNS rebinding attack detected: {} resolved to private IP {}",
                    host,
                    addr.ip()
                )));
            }
            if safe_addr.is_none() {
                safe_addr = Some(*addr);
            }
        }

        // safe_addr 一定有值，因为 addrs 非空且所有 IP 都通过了私网检查
        Ok(safe_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_check_valid() {
        assert!(UrlSecurityChecker::check_url("https://api.anthropic.com").is_ok());
        assert!(UrlSecurityChecker::check_url("https://example.com/path").is_ok());
    }

    #[test]
    fn test_url_check_localhost() {
        assert!(UrlSecurityChecker::check_url("http://localhost").is_err());
        assert!(UrlSecurityChecker::check_url("http://127.0.0.1").is_err());
        assert!(UrlSecurityChecker::check_url("http://[::1]").is_err());
    }

    #[test]
    fn test_url_check_private_ip() {
        assert!(UrlSecurityChecker::check_url("http://192.168.1.1").is_err());
        assert!(UrlSecurityChecker::check_url("http://10.0.0.1").is_err());
        assert!(UrlSecurityChecker::check_url("http://172.16.0.1").is_err());
    }

    #[test]
    fn test_url_check_internal_domain() {
        assert!(UrlSecurityChecker::check_url("http://server.local").is_err());
        assert!(UrlSecurityChecker::check_url("http://api.internal").is_err());
        assert!(UrlSecurityChecker::check_url("http://test.localhost").is_err());
    }

    #[test]
    fn test_url_check_unsupported_scheme() {
        assert!(UrlSecurityChecker::check_url("file:///etc/passwd").is_err());
        assert!(UrlSecurityChecker::check_url("ftp://ftp.example.com").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        use std::net::Ipv4Addr;

        assert!(UrlSecurityChecker::is_private_ip(&IpAddr::V4(Ipv4Addr::new(
            192, 168, 1, 1
        ))));
        assert!(UrlSecurityChecker::is_private_ip(&IpAddr::V4(Ipv4Addr::new(
            10, 0, 0, 1
        ))));
        assert!(UrlSecurityChecker::is_private_ip(&IpAddr::V4(Ipv4Addr::new(
            127, 0, 0, 1
        ))));
        assert!(!UrlSecurityChecker::is_private_ip(&IpAddr::V4(Ipv4Addr::new(
            8, 8, 8, 8
        ))));
    }
}
