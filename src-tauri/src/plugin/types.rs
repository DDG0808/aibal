// 插件系统类型定义
// 基于 contracts/types/ 目录中的 TypeScript 类型定义
// Phase 2+ 预留类型，当前未使用
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ============================================================================
// 基础类型
// ============================================================================

/// 操作结果包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

impl<T> Result<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: AppError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// 空结果类型
pub type EmptyResult = Result<()>;

/// 应用错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    /// 错误码
    pub code: String,
    /// 用户可读消息
    pub message: String,
    /// 详细信息 (用于调试)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

// ============================================================================
// 插件类型
// ============================================================================

/// 插件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Data,
    Event,
    Hybrid,
}

/// 数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Usage,
    Balance,
    Status,
    Custom,
}

/// 插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    /// 插件 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 数据类型 (DataPlugin/HybridPlugin)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DataType>,
    /// 是否启用
    pub enabled: bool,
    /// 是否健康
    pub healthy: bool,
    /// 作者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 图标文件名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// 更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 插件 ID
    pub id: String,
    /// 当前版本
    pub current_version: String,
    /// 最新版本
    pub latest_version: String,
    /// 更新说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    /// 下载地址
    pub download_url: String,
    /// 插件包文件哈希
    pub sha256: String,
    /// manifest.json 签名
    pub signature: String,
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// 是否有效
    pub valid: bool,
    /// 错误消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 字段级错误
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<std::collections::HashMap<String, String>>,
}

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 插件健康信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHealth {
    /// 插件 ID
    pub plugin_id: String,
    /// 健康状态
    pub status: HealthStatus,
    /// 最后成功时间 (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success: Option<String>,
    /// 最后错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// 错误计数（累计）
    pub error_count: u32,
    /// 平均延迟 (ms) - 基于滑动窗口
    pub avg_latency_ms: f64,
    /// P99 延迟 (ms) - 基于滑动窗口
    pub p99_latency_ms: f64,
    /// 成功率 (0-1) - 基于滑动窗口
    pub success_rate: f64,
    /// 总调用次数
    pub total_calls: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
}

// ============================================================================
// 插件数据类型
// ============================================================================

/// 插件数据基础结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDataBase {
    /// 数据来源插件
    pub plugin_id: String,
    /// 最后更新时间 (ISO 8601)
    pub last_updated: String,
}

/// Usage 维度
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDimension {
    /// 维度 ID
    pub id: String,
    /// 显示标签
    pub label: String,
    /// 使用百分比
    pub percentage: f64,
    /// 已用量
    pub used: f64,
    /// 限额
    pub limit: f64,
    /// 重置时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_time: Option<String>,
}

/// 使用量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageData {
    #[serde(flatten)]
    pub base: PluginDataBase,
    // 注意: data_type 由 PluginData enum 的 #[serde(tag = "dataType")] 自动提供
    /// 使用百分比 (0-100)
    pub percentage: f64,
    /// 已用量
    pub used: f64,
    /// 限额
    pub limit: f64,
    /// 单位
    pub unit: String,
    /// 重置时间 (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_time: Option<String>,
    /// 重置标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_label: Option<String>,
    /// 多维度使用量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<UsageDimension>>,
}

/// 余额数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceData {
    #[serde(flatten)]
    pub base: PluginDataBase,
    // 注意: data_type 由 PluginData enum 的 #[serde(tag = "dataType")] 自动提供
    /// 余额
    pub balance: f64,
    /// 货币
    pub currency: String,
    /// 总额度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<f64>,
    /// 已用额度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_quota: Option<f64>,
    /// 到期时间 (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// 状态指示器
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusIndicator {
    None,
    Minor,
    Major,
    Critical,
    Unknown,
}

/// 状态数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusData {
    #[serde(flatten)]
    pub base: PluginDataBase,
    // 注意: data_type 由 PluginData enum 的 #[serde(tag = "dataType")] 自动提供
    /// 状态指示
    pub indicator: StatusIndicator,
    /// 状态描述
    pub description: String,
}

/// 自定义数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomData {
    #[serde(flatten)]
    pub base: PluginDataBase,
    // 注意: data_type 由 PluginData enum 的 #[serde(tag = "dataType")] 自动提供
    /// 自定义渲染 HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_html: Option<String>,
    /// 自定义数据
    pub payload: serde_json::Value,
    /// 卡片标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 卡片副标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
}

/// 插件数据联合类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "dataType", rename_all = "lowercase")]
pub enum PluginData {
    Usage(UsageData),
    Balance(BalanceData),
    Status(StatusData),
    Custom(CustomData),
}

// ============================================================================
// 错误类型
// ============================================================================

/// 插件错误类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginErrorType {
    NetworkError,
    AuthError,
    RateLimit,
    Timeout,
    ParseError,
    ProviderError,
    SandboxLimit,
    PermissionDenied,
    StorageLimit,
    CacheError,
    IncompatibleApiVersion,
    Unknown,
}

impl PluginErrorType {
    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PluginErrorType::NetworkError
                | PluginErrorType::Timeout
                | PluginErrorType::RateLimit
                | PluginErrorType::ProviderError
                | PluginErrorType::StorageLimit
                | PluginErrorType::CacheError
        )
    }

    /// 从 HTTP 状态码推断错误类型
    pub fn from_http_status(status: u16) -> Self {
        match status {
            401 | 403 => PluginErrorType::AuthError,
            429 => PluginErrorType::RateLimit,
            408 | 504 => PluginErrorType::Timeout,
            500..=599 => PluginErrorType::ProviderError,
            _ => PluginErrorType::Unknown,
        }
    }
}

/// 插件错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginError {
    /// 错误类型
    #[serde(rename = "type")]
    pub error_type: PluginErrorType,
    /// 错误消息
    pub message: String,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl PluginError {
    pub fn new(error_type: PluginErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.error_type, self.message)
    }
}

impl std::error::Error for PluginError {}
