// Phase 5A: 安全工具链
// 提供签名验证、完整性校验、安全解压功能

pub mod canonical;
pub mod extractor;
pub mod integrity;
pub mod signature;

use std::io;
use std::path::PathBuf;

// ============================================================================
// 统一错误类型
// ============================================================================

/// 安全模块错误类型
#[derive(Debug)]
pub enum SecurityError {
    /// 签名验证失败
    SignatureInvalid {
        reason: String,
    },
    /// 签名格式错误
    SignatureFormatError {
        expected: &'static str,
        got: String,
    },
    /// 公钥未找到
    PublicKeyNotFound {
        key_id: String,
    },
    /// 哈希不匹配
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    /// 文件缺失
    FileMissing {
        file: String,
    },
    /// 路径穿越攻击
    PathTraversal {
        path: String,
    },
    /// 符号链接被拒绝
    SymlinkRejected {
        path: String,
    },
    /// 文件大小超限
    FileTooLarge {
        file: String,
        size: u64,
        limit: u64,
    },
    /// 总大小超限
    TotalSizeTooLarge {
        total: u64,
        limit: u64,
    },
    /// 不允许的文件类型
    FileTypeNotAllowed {
        file: String,
        extension: String,
    },
    /// 条目数过多
    TooManyEntries {
        count: usize,
        limit: usize,
    },
    /// IO 错误
    IoError(io::Error),
    /// ZIP 错误
    ZipError(String),
    /// JSON 解析错误
    JsonError(serde_json::Error),
    /// Base64 解码错误
    Base64Error(String),
    /// 原子替换失败
    AtomicReplaceFailed {
        source: PathBuf,
        target: PathBuf,
        reason: String,
    },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignatureInvalid { reason } => {
                write!(f, "签名验证失败: {}", reason)
            }
            Self::SignatureFormatError { expected, got } => {
                write!(f, "签名格式错误: 期望 '{}', 实际 '{}'", expected, got)
            }
            Self::PublicKeyNotFound { key_id } => {
                write!(f, "公钥未找到: {}", key_id)
            }
            Self::HashMismatch {
                file,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "文件哈希不匹配: {} (期望 {}, 实际 {})",
                    file, expected, actual
                )
            }
            Self::FileMissing { file } => {
                write!(f, "文件缺失: {}", file)
            }
            Self::PathTraversal { path } => {
                write!(f, "路径穿越攻击被阻止: {}", path)
            }
            Self::SymlinkRejected { path } => {
                write!(f, "符号链接被拒绝: {}", path)
            }
            Self::FileTooLarge { file, size, limit } => {
                write!(
                    f,
                    "文件过大: {} ({} bytes, 限制 {} bytes)",
                    file, size, limit
                )
            }
            Self::TotalSizeTooLarge { total, limit } => {
                write!(f, "总大小过大: {} bytes (限制 {} bytes)", total, limit)
            }
            Self::FileTypeNotAllowed { file, extension } => {
                write!(f, "不允许的文件类型: {} (扩展名: {})", file, extension)
            }
            Self::TooManyEntries { count, limit } => {
                write!(f, "条目数过多: {} (限制 {})", count, limit)
            }
            Self::IoError(e) => write!(f, "IO 错误: {}", e),
            Self::ZipError(e) => write!(f, "ZIP 错误: {}", e),
            Self::JsonError(e) => write!(f, "JSON 错误: {}", e),
            Self::Base64Error(e) => write!(f, "Base64 错误: {}", e),
            Self::AtomicReplaceFailed {
                source,
                target,
                reason,
            } => {
                write!(
                    f,
                    "原子替换失败: {:?} -> {:?} ({})",
                    source, target, reason
                )
            }
        }
    }
}

impl std::error::Error for SecurityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::JsonError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for SecurityError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<serde_json::Error> for SecurityError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

impl From<zip::result::ZipError> for SecurityError {
    fn from(err: zip::result::ZipError) -> Self {
        Self::ZipError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SecurityError>;

// ============================================================================
// 常量定义
// ============================================================================

/// 单文件大小限制: 10MB
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// 总解压大小限制: 50MB
pub const MAX_TOTAL_SIZE: u64 = 50 * 1024 * 1024;

/// 最大条目数限制: 1000 (防止 ZIP 炸弹)
pub const MAX_ENTRIES: usize = 1000;

/// 允许的文件扩展名
pub const ALLOWED_EXTENSIONS: &[&str] = &["js", "json", "png", "svg"];

/// 备份保留版本数
pub const BACKUP_VERSIONS: usize = 3;

// ============================================================================
// 重新导出
// ============================================================================

pub use canonical::canonicalize;
pub use extractor::SecureExtractor;
pub use integrity::{calculate_sha256, verify_file_hash, verify_manifest_files};
pub use signature::{verify_manifest_signature, SignatureVerifier};
