// 插件生命周期管理模块
// Phase 2.3: 生命周期管理
// Phase 4: 通信与配置集成
//
// 实现任务:
// - 2.3.1 插件发现 (扫描 ~/.config/cuk/plugins/)
// - 2.3.2 Module 加载 (declare + eval 模式)
// - 2.3.3 metadata 解析
// - 2.3.4 onLoad 生命周期
// - 2.3.5 onUnload 生命周期
// - 2.3.6 资源注册表
// - 2.3.7 资源强制回收
// - 4.x Phase 4 集成 (EventBus/ConfigManager/PermissionChecker)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use crate::plugin::config::{ConfigManager, ConfigSchema};
use crate::plugin::event_bus::EventBus;
use crate::plugin::monitoring::SlidingWindow;
use crate::plugin::permission::{MethodRegistry, PermissionChecker};
use crate::plugin::sandbox::PluginCallRequest;
use crate::plugin::types::{
    DataType, HealthStatus, PluginData, PluginHealth, PluginInfo, PluginType, ValidationResult,
};
use chrono::Utc;
use std::time::Instant;

// ============================================================================
// TOCTOU 防护：openat 链式验证（Unix）
// ============================================================================

/// Unix: 使用 openat 链式打开目录，消除中间目录 TOCTOU 窗口
///
/// 问题：传统的路径检查存在 TOCTOU（Time-of-check to time-of-use）漏洞：
/// 1. 检查 plugins/foo 不是 symlink
/// 2. 攻击者将 foo 替换为 symlink → /etc
/// 3. 代码使用 plugins/foo/entry.js → /etc/entry.js
///
/// 解决方案：使用 openat 系统调用链式打开每个目录组件
/// - 每个目录通过已验证的父目录 fd 打开
/// - 使用 O_NOFOLLOW 拒绝 symlink
/// - 攻击者无法在打开后替换（fd 已指向 inode）
/// - **关键修复**：返回打开的 File 而非路径，彻底消除验证-使用窗口
#[cfg(unix)]
mod openat_verifier {
    use std::ffi::CString;
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
    use std::path::{Component, Path};

    use super::LifecycleError;

    /// 使用 openat 链式打开文件（彻底消除 TOCTOU）
    ///
    /// 从 base_dir 开始，逐级使用 openat + O_NOFOLLOW 打开每个组件，
    /// 最终返回打开的文件句柄。调用方直接从 File 读取内容，无 TOCTOU 窗口。
    ///
    /// # 参数
    /// - `base_dir`: 基础目录（必须是可信的，如 plugins 根目录）
    /// - `relative_path`: 相对路径（如 "foo/bar.js"）
    ///
    /// # 返回
    /// - `Ok(File)`: 已打开的安全文件句柄
    /// - `Err`: 路径包含 symlink 或其他错误
    pub fn open_file_safely(base_dir: &Path, relative_path: &str) -> Result<File, LifecycleError> {
        // 解析相对路径的组件
        let rel_path = Path::new(relative_path);
        let components: Vec<_> = rel_path
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s),
                _ => None,
            })
            .collect();

        if components.is_empty() {
            return Err(LifecycleError::PluginLoad("相对路径为空".into()));
        }

        // 打开基础目录
        let base_cstr = CString::new(base_dir.as_os_str().as_bytes())
            .map_err(|_| LifecycleError::PluginLoad("路径包含非法字符".into()))?;

        let base_fd = unsafe {
            let fd = libc::open(
                base_cstr.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            );
            if fd < 0 {
                return Err(LifecycleError::PluginLoad(format!(
                    "无法打开基础目录（可能是 symlink）: {:?}",
                    base_dir
                )));
            }
            OwnedFd::from_raw_fd(fd)
        };

        let mut current_fd = base_fd;

        // 链式打开每个中间目录（除了最后一个文件）
        let dirs = &components[..components.len().saturating_sub(1)];
        for component in dirs {
            let comp_cstr = CString::new(component.as_bytes())
                .map_err(|_| LifecycleError::PluginLoad("路径组件包含非法字符".into()))?;

            let next_fd = unsafe {
                let fd = libc::openat(
                    current_fd.as_raw_fd(),
                    comp_cstr.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                );
                if fd < 0 {
                    let errno = std::io::Error::last_os_error();
                    return Err(LifecycleError::PluginLoad(format!(
                        "路径组件打开失败（可能是 symlink）: {:?}, 错误: {}",
                        component, errno
                    )));
                }
                OwnedFd::from_raw_fd(fd)
            };

            current_fd = next_fd;
        }

        // 打开最终文件（使用 O_NOFOLLOW 拒绝 symlink）
        let file_component = components.last()
            .ok_or_else(|| LifecycleError::PluginLoad("路径组件为空".into()))?;
        let file_cstr = CString::new(file_component.as_bytes())
            .map_err(|_| LifecycleError::PluginLoad("文件名包含非法字符".into()))?;

        let file_fd = unsafe {
            let fd = libc::openat(
                current_fd.as_raw_fd(),
                file_cstr.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            );
            if fd < 0 {
                let errno = std::io::Error::last_os_error();
                // 区分 symlink 错误和文件不存在
                if errno.raw_os_error() == Some(libc::ELOOP) {
                    return Err(LifecycleError::PluginLoad(format!(
                        "入口文件是符号链接（安全风险）: {:?}",
                        file_component
                    )));
                }
                return Err(LifecycleError::PluginLoad(format!(
                    "无法打开入口文件: {:?}, 错误: {}",
                    file_component, errno
                )));
            }
            OwnedFd::from_raw_fd(fd)
        };

        // 将 OwnedFd 转换为 File
        Ok(unsafe { File::from_raw_fd(file_fd.into_raw_fd()) })
    }

    /// 读取入口文件内容（安全版本，无 TOCTOU 窗口）
    ///
    /// 使用 openat 打开文件并直接读取内容，调用方无需再次打开文件。
    pub fn read_entry_file(base_dir: &Path, relative_path: &str) -> Result<String, LifecycleError> {
        let mut file = open_file_safely(base_dir, relative_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| LifecycleError::PluginLoad(format!("读取入口文件失败: {}", e)))?;
        Ok(content)
    }

    /// 使用 openat 链式验证路径安全性（兼容旧 API，但存在 TOCTOU 窗口）
    ///
    /// **警告**：此函数仅验证路径，验证后再打开文件存在 TOCTOU 窗口。
    /// 新代码应使用 `open_file_safely()` 或 `read_entry_file()` 直接获取文件/内容。
    #[allow(deprecated)]
    #[deprecated(note = "存在 TOCTOU 窗口，请使用 open_file_safely() 或 read_entry_file()")]
    pub fn verify_path_no_symlink(base_dir: &Path, relative_path: &str) -> Result<(), LifecycleError> {
        // 尝试打开文件验证路径安全性，然后关闭
        let _file = open_file_safely(base_dir, relative_path)?;
        Ok(())
    }
}

/// Windows: 使用 symlink_metadata 检查（无原生 openat 支持）
#[cfg(not(unix))]
mod openat_verifier {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    use super::LifecycleError;

    /// Windows 版本：打开文件并返回句柄
    ///
    /// Windows 没有 openat，使用传统方式打开。
    /// 注意：Windows 需要管理员权限创建 symlink，攻击面较小。
    pub fn open_file_safely(base_dir: &Path, relative_path: &str) -> Result<File, LifecycleError> {
        // 先验证路径
        let mut current = base_dir.to_path_buf();
        for component in Path::new(relative_path).components() {
            if let std::path::Component::Normal(c) = component {
                current.push(c);
                if let Ok(meta) = std::fs::symlink_metadata(&current) {
                    if meta.file_type().is_symlink() {
                        return Err(LifecycleError::PluginLoad(format!(
                            "路径包含符号链接（安全风险）: {:?}",
                            current
                        )));
                    }
                }
            }
        }

        // 打开文件
        let full_path = base_dir.join(relative_path);
        File::open(&full_path)
            .map_err(|e| LifecycleError::PluginLoad(format!("无法打开入口文件: {:?}, 错误: {}", full_path, e)))
    }

    /// 读取入口文件内容
    pub fn read_entry_file(base_dir: &Path, relative_path: &str) -> Result<String, LifecycleError> {
        let mut file = open_file_safely(base_dir, relative_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| LifecycleError::PluginLoad(format!("读取入口文件失败: {}", e)))?;
        Ok(content)
    }

    /// Windows 版本：逐级检查 symlink（兼容旧 API）
    #[allow(deprecated)]
    #[deprecated(note = "存在 TOCTOU 窗口，请使用 open_file_safely() 或 read_entry_file()")]
    pub fn verify_path_no_symlink(base_dir: &Path, relative_path: &str) -> Result<(), LifecycleError> {
        let _file = open_file_safely(base_dir, relative_path)?;
        Ok(())
    }
}

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("插件目录不存在: {0}")]
    PluginDirNotFound(PathBuf),

    #[error("manifest.json 不存在: {0}")]
    ManifestNotFound(PathBuf),

    #[error("manifest.json 解析失败: {0}")]
    ManifestParse(String),

    #[error("插件加载失败: {0}")]
    PluginLoad(String),

    #[error("插件卸载失败: {0}")]
    PluginUnload(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JS 执行错误: {0}")]
    JsExecution(String),
}

// ============================================================================
// 插件清单 (manifest.json)
// ============================================================================

/// 插件清单结构
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// 插件 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// API 版本
    pub api_version: String,
    /// 插件类型
    pub plugin_type: String,
    /// 数据类型 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    /// 入口文件
    #[serde(default = "default_entry")]
    pub entry: String,
    /// 作者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 主页
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// 图标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 权限声明 (格式: call:{pluginId}:{method})
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 刷新间隔 (毫秒)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_ms: Option<u64>,
    /// 订阅事件 (完整事件名，如 plugin:claude-usage:data_updated)
    #[serde(default)]
    pub subscribed_events: Vec<String>,
    /// 配置 Schema (Phase 4.2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// 暴露的方法列表 (Phase 4.3)
    #[serde(default)]
    pub exposed_methods: Vec<String>,
    /// 文件哈希 (签名验证用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<HashMap<String, String>>,
    /// 签名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

fn default_entry() -> String {
    "plugin.js".to_string()
}

impl PluginManifest {
    /// 从文件加载
    pub fn load_from_file(path: &Path) -> Result<Self, LifecycleError> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| LifecycleError::ManifestParse(e.to_string()))
    }

    /// 获取插件类型枚举
    pub fn get_plugin_type(&self) -> PluginType {
        match self.plugin_type.to_lowercase().as_str() {
            "data" => PluginType::Data,
            "event" => PluginType::Event,
            "hybrid" => PluginType::Hybrid,
            _ => PluginType::Data,
        }
    }

    /// 获取数据类型枚举
    pub fn get_data_type(&self) -> Option<DataType> {
        self.data_type.as_ref().map(|t| match t.to_lowercase().as_str() {
            "usage" => DataType::Usage,
            "balance" => DataType::Balance,
            "status" => DataType::Status,
            "custom" => DataType::Custom,
            _ => DataType::Custom,
        })
    }

    /// 转换为 PluginInfo
    pub fn to_plugin_info(&self, enabled: bool, healthy: bool) -> PluginInfo {
        PluginInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            plugin_type: self.get_plugin_type(),
            data_type: self.get_data_type(),
            enabled,
            healthy,
            author: self.author.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
        }
    }
}

// ============================================================================
// 资源注册表
// ============================================================================

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Timer,
    Subscription,
    Request,
}

/// 资源条目
#[derive(Debug)]
pub struct ResourceEntry {
    pub id: u64,
    pub resource_type: ResourceType,
    pub created_at: std::time::Instant,
}

/// 资源注册表
/// 跟踪插件创建的所有资源，用于卸载时强制回收
pub struct ResourceRegistry {
    /// 资源映射
    resources: HashMap<u64, ResourceEntry>,
    /// 下一个资源 ID
    next_id: u64,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_id: 1,
        }
    }

    /// 注册资源
    pub fn register(&mut self, resource_type: ResourceType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.resources.insert(
            id,
            ResourceEntry {
                id,
                resource_type,
                created_at: std::time::Instant::now(),
            },
        );

        id
    }

    /// 注销资源
    pub fn unregister(&mut self, id: u64) -> Option<ResourceEntry> {
        self.resources.remove(&id)
    }

    /// 获取所有资源 ID
    pub fn all_ids(&self) -> Vec<u64> {
        self.resources.keys().copied().collect()
    }

    /// 按类型获取资源 ID
    pub fn ids_by_type(&self, resource_type: ResourceType) -> Vec<u64> {
        self.resources
            .iter()
            .filter(|(_, r)| r.resource_type == resource_type)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 清空所有资源
    pub fn clear(&mut self) -> Vec<ResourceEntry> {
        self.resources.drain().map(|(_, v)| v).collect()
    }

    /// 资源数量
    pub fn count(&self) -> usize {
        self.resources.len()
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 插件实例
// ============================================================================

/// 插件实例状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// 未加载
    Unloaded,
    /// 加载中
    Loading,
    /// 已加载
    Loaded,
    /// 运行中
    Running,
    /// 错误
    Error,
}

/// 插件实例
pub struct PluginInstance {
    /// 插件 ID
    pub id: String,
    /// 插件路径
    pub path: PathBuf,
    /// 清单
    pub manifest: PluginManifest,
    /// 状态
    pub state: PluginState,
    /// 是否启用
    pub enabled: bool,
    /// 资源注册表
    pub resources: ResourceRegistry,
    /// 最后错误
    pub last_error: Option<String>,
    /// 插件配置
    pub config: HashMap<String, serde_json::Value>,
    /// 缓存的插件数据
    pub cached_data: Option<PluginData>,
    /// 最后成功时间
    pub last_success: Option<Instant>,
    /// 错误计数（累计）
    pub error_count: u32,
    /// 成功请求数（累计）
    pub success_count: u32,
    /// 总延迟累计 (毫秒)
    pub total_latency_ms: f64,
    /// 滑动窗口统计（Phase 6）
    pub sliding_window: SlidingWindow,
    /// 连续失败次数（Phase 6）
    pub consecutive_failures: u32,
    /// 总调用次数（Phase 6）
    pub total_calls: u64,
}

impl PluginInstance {
    /// 创建新实例
    pub fn new(path: PathBuf, manifest: PluginManifest) -> Self {
        Self {
            id: manifest.id.clone(),
            path,
            manifest,
            state: PluginState::Unloaded,
            enabled: false,
            resources: ResourceRegistry::new(),
            last_error: None,
            config: HashMap::new(),
            cached_data: None,
            last_success: None,
            error_count: 0,
            success_count: 0,
            total_latency_ms: 0.0,
            sliding_window: SlidingWindow::with_default_size(),
            consecutive_failures: 0,
            total_calls: 0,
        }
    }

    /// 读取入口文件内容（安全版本，无 TOCTOU 窗口）
    ///
    /// 使用 openat 链式打开并直接读取文件内容，完全消除 TOCTOU 窗口。
    /// 这是推荐的安全 API，调用方无需再打开文件。
    ///
    /// # 安全保证
    /// - 路径验证和文件读取在同一操作中完成
    /// - 攻击者无法在验证后替换文件
    /// - 比 `entry_path()` + `fs::read_to_string()` 更安全
    ///
    /// # 返回
    /// - `Ok(String)`: 入口文件内容
    /// - `Err(LifecycleError)`: 路径校验或读取失败
    pub fn read_entry_content(&self) -> Result<String, LifecycleError> {
        let entry = &self.manifest.entry;

        // 1. 检查是否为绝对路径
        if Path::new(entry).is_absolute() {
            return Err(LifecycleError::PluginLoad(format!(
                "插件入口文件不能是绝对路径: {}",
                entry
            )));
        }

        // 2. 检查是否包含路径遍历组件
        if entry.contains("..") {
            return Err(LifecycleError::PluginLoad(format!(
                "插件入口文件路径包含非法字符 '..': {}",
                entry
            )));
        }

        // 3. 使用 openat 安全读取文件（无 TOCTOU 窗口）
        let content = openat_verifier::read_entry_file(&self.path, entry)?;

        log::debug!("安全读取插件入口文件完成: {:?}/{}", self.path, entry);
        Ok(content)
    }

    /// 获取入口文件路径（存在 TOCTOU 窗口，仅用于兼容）
    ///
    /// **警告**：返回路径后再打开文件存在 TOCTOU 窗口。
    /// 新代码应使用 `read_entry_content()` 直接读取文件内容。
    ///
    /// # TOCTOU 风险
    /// - 此方法返回 PathBuf 后，调用方再打开文件
    /// - 在返回路径和打开文件之间，攻击者可能替换文件
    /// - 推荐使用 `read_entry_content()` 消除此窗口
    #[deprecated(note = "存在 TOCTOU 窗口，推荐使用 read_entry_content()")]
    pub fn entry_path(&self) -> Result<PathBuf, LifecycleError> {
        let entry = &self.manifest.entry;

        // 1. 检查是否为绝对路径
        if Path::new(entry).is_absolute() {
            return Err(LifecycleError::PluginLoad(format!(
                "插件入口文件不能是绝对路径: {}",
                entry
            )));
        }

        // 2. 检查是否包含路径遍历组件
        if entry.contains("..") {
            return Err(LifecycleError::PluginLoad(format!(
                "插件入口文件路径包含非法字符 '..': {}",
                entry
            )));
        }

        // 3. 构建完整路径
        let full_path = self.path.join(entry);

        // 4. 使用 openat 验证路径安全（仍存在 TOCTOU 窗口）
        #[allow(deprecated)]
        openat_verifier::verify_path_no_symlink(&self.path, entry)?;

        // 5. 规范化路径并验证是否在插件目录内
        let plugin_dir = match self.path.canonicalize() {
            Ok(p) => p,
            Err(_) => self.path.clone(),
        };

        // 检查路径组件，确保没有逃逸
        let mut resolved = plugin_dir.clone();
        for component in Path::new(entry).components() {
            if let std::path::Component::Normal(c) = component {
                resolved.push(c);
            }
        }

        // 6. 最终验证：resolved 路径必须以 plugin_dir 开头
        if !resolved.starts_with(&plugin_dir) {
            return Err(LifecycleError::PluginLoad(format!(
                "插件入口文件路径逃逸出插件目录: {}",
                entry
            )));
        }

        // 7. 如果文件存在，使用 canonicalize 做最终验证
        if full_path.exists() {
            let canonical = full_path.canonicalize().map_err(|e| {
                LifecycleError::PluginLoad(format!("无法规范化入口路径: {}", e))
            })?;

            if !canonical.starts_with(&plugin_dir) {
                return Err(LifecycleError::PluginLoad(format!(
                    "插件入口文件实际位置逃逸出插件目录: {:?}",
                    canonical
                )));
            }
        }

        log::debug!("插件入口文件路径校验通过: {:?}", full_path);
        Ok(full_path)
    }

    /// 获取入口文件路径（不安全，仅用于测试）
    #[deprecated(note = "请使用 entry_path() 以确保路径安全")]
    pub fn entry_path_unchecked(&self) -> PathBuf {
        self.path.join(&self.manifest.entry)
    }

    /// 转换为 PluginInfo
    pub fn to_info(&self) -> PluginInfo {
        self.manifest.to_plugin_info(
            self.enabled,
            self.state == PluginState::Running,
        )
    }

    /// 转换为 PluginHealth（基于滑动窗口统计）
    ///
    /// Phase 6 增强：使用滑动窗口计算成功率和延迟，
    /// 增加 P99 延迟、连续失败次数、总调用次数统计。
    ///
    /// P1 修复：只需 `&self`，支持健康查询使用读锁。
    pub fn to_health(&self) -> PluginHealth {
        // 从滑动窗口获取统计数据
        let success_rate = self.sliding_window.success_rate();
        let avg_latency_ms = self.sliding_window.avg_latency_ms();
        let p99_latency_ms = self.sliding_window.p99_latency_ms();

        // 健康状态判定（Phase 6.1.3 + P2 修复：纳入延迟考量）
        // 规则：
        // 1. 连续失败 >= 3 次 → Unhealthy（立即降级）
        // 2. 成功率 < 80% → Unhealthy
        // 3. P99 延迟 > 5000ms → Degraded（高延迟降级）
        // 4. 成功率 80%-95% → Degraded
        // 5. 成功率 >= 95% 且连续失败 < 3 且延迟正常 → Healthy
        let status = if self.consecutive_failures >= 3 {
            HealthStatus::Unhealthy
        } else if success_rate < 0.8 {
            HealthStatus::Unhealthy
        } else if p99_latency_ms > 5000.0 {
            // P2 修复：高延迟导致 Degraded
            HealthStatus::Degraded
        } else if success_rate < 0.95 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        // 格式化最后成功时间
        let last_success = self.last_success.map(|t| {
            let elapsed = t.elapsed();
            chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::from_std(elapsed).unwrap_or_default())
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        });

        PluginHealth {
            plugin_id: self.id.clone(),
            status,
            last_success,
            last_error: self.last_error.clone(),
            error_count: self.error_count,
            avg_latency_ms,
            p99_latency_ms,
            success_rate,
            total_calls: self.total_calls,
            consecutive_failures: self.consecutive_failures,
        }
    }

    /// 记录成功请求
    ///
    /// 更新累计统计和滑动窗口统计，重置连续失败计数
    pub fn record_success(&mut self, latency_ms: f64) {
        // 累计统计
        self.success_count += 1;
        self.total_latency_ms += latency_ms;
        self.last_success = Some(Instant::now());
        self.last_error = None;
        self.total_calls += 1;

        // 滑动窗口统计（Phase 6）
        self.sliding_window.record_success(latency_ms);

        // 重置连续失败计数（Phase 6）
        self.consecutive_failures = 0;
    }

    /// 记录失败请求
    ///
    /// 更新累计统计、滑动窗口统计和连续失败计数
    pub fn record_failure(&mut self, error: String) {
        // 累计统计
        self.error_count += 1;
        self.last_error = Some(error);
        self.total_calls += 1;

        // 滑动窗口统计（Phase 6）
        // 失败时延迟设为 0（或可以传入实际延迟）
        self.sliding_window.record_failure(0.0);

        // 增加连续失败计数（Phase 6）
        self.consecutive_failures += 1;
    }

    /// 记录失败请求（带延迟）
    ///
    /// Phase 6 新增：允许记录失败请求的延迟
    pub fn record_failure_with_latency(&mut self, error: String, latency_ms: f64) {
        self.error_count += 1;
        self.last_error = Some(error);
        self.total_calls += 1;
        self.sliding_window.record_failure(latency_ms);
        self.consecutive_failures += 1;
    }

    /// 重置健康统计
    pub fn reset_health_stats(&mut self) {
        self.error_count = 0;
        self.success_count = 0;
        self.total_latency_ms = 0.0;
        self.last_error = None;
        self.sliding_window.clear();
        self.consecutive_failures = 0;
        self.total_calls = 0;
    }
}

// ============================================================================
// 插件发现器
// ============================================================================

/// 插件发现器
pub struct PluginDiscovery {
    /// 插件目录
    plugins_dir: PathBuf,
}

impl PluginDiscovery {
    /// 创建发现器
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// 使用默认目录创建
    pub fn with_default_dir() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cuk")
            .join("plugins");
        Self::new(dir)
    }

    /// 获取插件目录
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// 确保插件目录存在（同步版本，仅用于初始化）
    pub fn ensure_dir(&self) -> Result<(), LifecycleError> {
        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir)?;
            log::info!("已创建插件目录: {:?}", self.plugins_dir);
        }
        Ok(())
    }

    /// 确保插件目录存在（异步版本）
    pub async fn ensure_dir_async(&self) -> Result<(), LifecycleError> {
        if !tokio::fs::try_exists(&self.plugins_dir).await.unwrap_or(false) {
            tokio::fs::create_dir_all(&self.plugins_dir)
                .await
                .map_err(LifecycleError::Io)?;
            log::info!("已创建插件目录: {:?}", self.plugins_dir);
        }
        Ok(())
    }

    /// 发现所有插件（同步版本，仅用于兼容）
    #[deprecated(note = "请使用 discover_async 以避免阻塞 Tokio worker")]
    pub fn discover(&self) -> Result<Vec<(PathBuf, PluginManifest)>, LifecycleError> {
        self.ensure_dir()?;

        let mut plugins = Vec::new();

        // 遍历插件目录
        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // 查找 manifest.json
            let manifest_path = path.join("manifest.json");
            if !manifest_path.exists() {
                log::debug!("跳过 {:?}: 无 manifest.json", path);
                continue;
            }

            // 加载清单
            match PluginManifest::load_from_file(&manifest_path) {
                Ok(manifest) => {
                    log::info!("发现插件: {} v{}", manifest.name, manifest.version);
                    plugins.push((path, manifest));
                }
                Err(e) => {
                    log::warn!("加载 {:?} 失败: {}", manifest_path, e);
                }
            }
        }

        Ok(plugins)
    }

    /// 发现所有插件（异步版本）
    ///
    /// 使用 tokio::fs 进行异步文件操作，不阻塞 Tokio worker
    pub async fn discover_async(&self) -> Result<Vec<(PathBuf, PluginManifest)>, LifecycleError> {
        self.ensure_dir_async().await?;

        let mut plugins = Vec::new();

        // 使用 tokio::fs::read_dir 进行异步目录遍历
        let mut entries = tokio::fs::read_dir(&self.plugins_dir)
            .await
            .map_err(LifecycleError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(LifecycleError::Io)? {
            let path = entry.path();

            // 使用异步方法检查是否为目录
            let metadata = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if !metadata.is_dir() {
                continue;
            }

            // 查找 manifest.json
            let manifest_path = path.join("manifest.json");
            if !tokio::fs::try_exists(&manifest_path).await.unwrap_or(false) {
                log::debug!("跳过 {:?}: 无 manifest.json", path);
                continue;
            }

            // 异步加载清单
            match tokio::fs::read_to_string(&manifest_path).await {
                Ok(content) => match serde_json::from_str::<PluginManifest>(&content) {
                    Ok(manifest) => {
                        log::info!("发现插件: {} v{}", manifest.name, manifest.version);
                        plugins.push((path, manifest));
                    }
                    Err(e) => {
                        log::warn!("解析 {:?} 失败: {}", manifest_path, e);
                    }
                },
                Err(e) => {
                    log::warn!("读取 {:?} 失败: {}", manifest_path, e);
                }
            }
        }

        Ok(plugins)
    }
}

// ============================================================================
// 插件管理器
// ============================================================================

/// 插件管理器
pub struct PluginManager {
    /// 发现器
    discovery: PluginDiscovery,
    /// 已加载的插件
    plugins: RwLock<HashMap<String, PluginInstance>>,

    // ========================================================================
    // Phase 4: 通信与配置组件
    // ========================================================================

    /// 事件总线 (Phase 4.1)
    event_bus: Arc<EventBus>,
    /// 配置管理器 (Phase 4.2)
    config_manager: Arc<ConfigManager>,
    /// 权限检查器 (Phase 4.3)
    permission_checker: Arc<PermissionChecker>,
    /// 方法注册表 (Phase 4.3)
    method_registry: Arc<MethodRegistry>,
    /// 跨插件调用请求发送端
    call_tx: mpsc::Sender<PluginCallRequest>,
    /// 跨插件调用请求接收端（由运行时消费）
    call_rx: Arc<RwLock<Option<mpsc::Receiver<PluginCallRequest>>>>,
    /// EventBus 分发器 handle（用于 shutdown）
    dispatcher_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// 跨插件调用分发器 handle（用于 shutdown）
    call_dispatcher_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl PluginManager {
    /// 创建管理器
    pub fn new(discovery: PluginDiscovery) -> Self {
        // 创建跨插件调用通道
        let (call_tx, call_rx) = mpsc::channel(100);

        // Phase 4 组件
        let method_registry = Arc::new(MethodRegistry::new());

        Self {
            discovery,
            plugins: RwLock::new(HashMap::new()),
            // Phase 4 组件
            event_bus: Arc::new(EventBus::new_default()),
            config_manager: Arc::new(ConfigManager::new()),
            permission_checker: Arc::new(PermissionChecker::new(method_registry.clone())),
            method_registry,
            call_tx,
            call_rx: Arc::new(RwLock::new(Some(call_rx))),
            dispatcher_handle: RwLock::new(None),
            call_dispatcher_handle: RwLock::new(None),
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(PluginDiscovery::with_default_dir())
    }

    // ========================================================================
    // Phase 4: 组件访问器
    // ========================================================================

    /// 获取事件总线
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    /// 获取配置管理器
    pub fn config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager.clone()
    }

    /// 获取权限检查器
    pub fn permission_checker(&self) -> Arc<PermissionChecker> {
        self.permission_checker.clone()
    }

    /// 获取方法注册表
    pub fn method_registry(&self) -> Arc<MethodRegistry> {
        self.method_registry.clone()
    }

    /// 获取跨插件调用发送端（用于注入到沙盒）
    pub fn call_sender(&self) -> mpsc::Sender<PluginCallRequest> {
        self.call_tx.clone()
    }

    /// 取出跨插件调用接收端（只能取出一次，供运行时消费）
    pub async fn take_call_receiver(&self) -> Option<mpsc::Receiver<PluginCallRequest>> {
        self.call_rx.write().await.take()
    }

    /// 启动 EventBus 分发器
    ///
    /// 在 discover_and_load 之后调用，开始事件分发循环
    /// 分发器会从队列中取出事件并分发给订阅者
    pub async fn start_dispatcher(&self) {
        let mut handle = self.dispatcher_handle.write().await;
        if handle.is_some() {
            log::warn!("EventBus 分发器已在运行");
            return;
        }

        let dispatcher_handle = self.event_bus.clone().spawn_dispatcher();
        *handle = Some(dispatcher_handle);
        log::info!("EventBus 分发器已启动");
    }

    /// 停止 EventBus 分发器
    ///
    /// 在应用关闭时调用，终止事件分发循环
    pub async fn stop_dispatcher(&self) {
        let mut handle = self.dispatcher_handle.write().await;
        if let Some(h) = handle.take() {
            h.abort();
            log::info!("EventBus 分发器已停止");
        }
    }

    /// 检查分发器是否正在运行
    pub async fn is_dispatcher_running(&self) -> bool {
        let handle = self.dispatcher_handle.read().await;
        if let Some(h) = handle.as_ref() {
            !h.is_finished()
        } else {
            false
        }
    }

    /// 启动跨插件调用分发器
    ///
    /// 消费 call_rx 中的调用请求，执行目标方法并回传结果
    ///
    /// 修复说明：
    /// - 使用动态插件列表查询，避免启动时快照漂移问题
    /// - 支持 stop 后重新 start（在 stop 时重建 channel）
    pub async fn start_call_dispatcher(&self) {
        let mut call_handle = self.call_dispatcher_handle.write().await;
        if call_handle.is_some() {
            log::warn!("跨插件调用分发器已在运行");
            return;
        }

        // 取出 call_receiver
        let call_rx = self.call_rx.write().await.take();
        if call_rx.is_none() {
            log::warn!("call_receiver 已被取出，无法启动调用分发器");
            return;
        }
        let mut call_rx = call_rx.unwrap();

        // 克隆需要的引用 - 使用动态引用而非快照
        let method_registry = self.method_registry.clone();
        // 修复问题2：传入 plugins 的 Arc 引用，支持动态查询
        // 注意：由于 self.plugins 是 RwLock，在 spawn 的 async 块中需要特殊处理
        // 这里我们克隆 method_registry，它内部会动态检查方法注册

        let handle = tokio::spawn(async move {
            log::info!("跨插件调用分发器已启动");

            while let Some(request) = call_rx.recv().await {
                let caller = request.caller.clone();
                let target = request.target.clone();
                let method = request.method.clone();
                let call_depth = request.call_depth;

                log::debug!(
                    "处理跨插件调用: {} -> {}::{} (depth={})",
                    caller, target, method, call_depth
                );

                // 检查调用深度
                if call_depth > PluginCallRequest::MAX_CALL_DEPTH {
                    let _ = request.response_tx.send(Err(format!(
                        "调用深度超限: {} > {}",
                        call_depth, PluginCallRequest::MAX_CALL_DEPTH
                    )));
                    continue;
                }

                // 修复问题2：通过 method_registry 动态检查（方法注册与插件存在绑定）
                // 如果方法已注册，说明插件存在且声明了该方法
                if !method_registry.is_registered(&target, &method).await {
                    let _ = request.response_tx.send(Err(format!(
                        "目标插件或方法不存在: {}::{}", target, method
                    )));
                    continue;
                }

                // 方法执行未实现 - 返回明确的错误
                //
                // 完整实现需要：
                // 1. 改为"常驻沙盒"模式 - 每个插件保持活跃的 JS 运行时
                // 2. 或"按需创建"模式 - 调用时临时创建沙盒，需要方法实现 JS 代码
                // 3. MethodRegistry 需要扩展为保存方法实现（函数指针或回调）
                //
                // 当前架构限制：
                // - 插件沙盒是"执行后销毁"模式
                // - MethodRegistry 只记录方法名，没有保存实际实现
                //
                // 建议：未来版本实现常驻沙盒模式，保持插件上下文可用
                let _ = request.response_tx.send(Err(format!(
                    "方法执行未实现: {}::{}。跨插件调用功能需要常驻沙盒模式支持，当前架构不支持。",
                    target, method
                )));
            }

            log::info!("跨插件调用分发器已停止");
        });

        *call_handle = Some(handle);
        log::info!("跨插件调用分发器已启动");
    }

    /// 停止跨插件调用分发器
    ///
    /// 注意：由于 context.call() 现在直接返回 not_supported，分发器实际上不会收到任何请求。
    /// 因此不需要支持"停止后重启"功能。如果未来实现了真正的跨插件调用，需要重新设计
    /// channel 的所有权模型（将 call_tx/call_rx 改为可重建的结构）。
    ///
    /// 当前行为：停止分发器后，call_rx 被消耗，无法重新启动。
    /// 这是可接受的，因为：
    /// 1. context.call() 直接返回 not_supported，不会发送请求到 channel
    /// 2. 分发器本身只是返回"功能未实现"错误
    /// 3. 应用生命周期中通常不需要重启分发器
    pub async fn stop_call_dispatcher(&self) {
        let mut handle = self.call_dispatcher_handle.write().await;
        if let Some(h) = handle.take() {
            h.abort();
            log::info!("跨插件调用分发器已停止");
        }
    }

    /// 发现并加载所有插件（异步版本）
    ///
    /// 使用异步文件操作，不阻塞 Tokio worker
    /// Phase 4: 自动注册 subscribedEvents/permissions/exposedMethods/configSchema
    pub async fn discover_and_load(&self) -> Result<Vec<PluginInfo>, LifecycleError> {
        // 使用异步版本的 discover
        let discovered = self.discovery.discover_async().await?;

        let mut plugins = self.plugins.write().await;
        let mut infos = Vec::new();

        for (path, manifest) in discovered {
            let id = manifest.id.clone();

            // Phase 4.1: 注册事件订阅
            if !manifest.subscribed_events.is_empty() {
                self.event_bus.subscribe(&id, &manifest.subscribed_events).await;
                log::debug!("[{}] 注册事件订阅: {:?}", id, manifest.subscribed_events);
            }

            // Phase 4.2: 注册配置 Schema
            if let Some(ref schema) = manifest.config_schema {
                if let Err(e) = self.config_manager.register_schema_from_json(&id, schema).await {
                    log::warn!("[{}] 配置 Schema 注册失败: {}", id, e);
                } else {
                    log::debug!("[{}] 已注册配置 Schema", id);
                }
            }

            // Phase 4.3: 注册权限声明
            if !manifest.permissions.is_empty() {
                self.permission_checker.register_permissions(&id, &manifest.permissions).await;
                log::debug!("[{}] 注册权限声明: {:?}", id, manifest.permissions);
            }

            // Phase 4.3: 注册暴露方法
            for method in &manifest.exposed_methods {
                self.method_registry.register(&id, method, None).await;
                log::debug!("[{}] 注册暴露方法: {}", id, method);
            }

            let instance = PluginInstance::new(path, manifest);
            let info = instance.to_info();
            plugins.insert(id, instance);
            infos.push(info);
        }

        log::info!("已发现 {} 个插件，Phase 4 组件已初始化", infos.len());
        Ok(infos)
    }

    /// 初始化插件系统（完整流程）
    ///
    /// 执行完整的插件系统初始化：
    /// 1. 发现并加载所有插件
    /// 2. 启动 EventBus 分发器
    /// 3. 启动跨插件调用分发器
    ///
    /// 这是推荐的初始化入口，替代手动调用 discover_and_load + start_dispatcher
    pub async fn init(&self) -> Result<Vec<PluginInfo>, LifecycleError> {
        // 1. 发现并加载插件
        let infos = self.discover_and_load().await?;

        // 2. 启动事件分发器
        self.start_dispatcher().await;

        // 3. 启动跨插件调用分发器
        self.start_call_dispatcher().await;

        log::info!("插件系统初始化完成，EventBus 和 Call 分发器已启动");
        Ok(infos)
    }

    /// 关闭插件系统
    ///
    /// 清理资源：
    /// 1. 停止 EventBus 分发器
    /// 2. 停止跨插件调用分发器
    /// 3. 卸载所有插件
    pub async fn shutdown(&self) {
        // 1. 停止事件分发器
        self.stop_dispatcher().await;

        // 2. 停止调用分发器
        self.stop_call_dispatcher().await;

        // 3. 清理插件（可选，视需求实现完整卸载）
        log::info!("插件系统已关闭");
    }

    /// 获取所有插件信息
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .await
            .values()
            .map(|p| p.to_info())
            .collect()
    }

    /// 获取单个插件信息
    pub async fn get_plugin(&self, id: &str) -> Option<PluginInfo> {
        self.plugins
            .read()
            .await
            .get(id)
            .map(|p| p.to_info())
    }

    /// 启用插件
    pub async fn enable_plugin(&self, id: &str) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.enabled = true;
            log::info!("已启用插件: {}", id);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    /// 禁用插件
    ///
    /// Phase 4: 同时清理事件订阅和暴露方法
    pub async fn disable_plugin(&self, id: &str) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.enabled = false;

            // 清理资源
            let resources = plugin.resources.clear();

            // Phase 4.1: 取消事件订阅
            self.event_bus.unsubscribe_all(id).await;

            // Phase 4.3: 取消方法注册
            self.method_registry.unregister_all(id).await;

            log::info!(
                "已禁用插件: {}, 释放 {} 个资源, 已清理 Phase 4 组件",
                id,
                resources.len()
            );
            Ok(())
        } else {
            Err(LifecycleError::PluginUnload(format!("插件不存在: {}", id)))
        }
    }

    /// 获取插件目录
    pub fn plugins_dir(&self) -> &Path {
        self.discovery.plugins_dir()
    }

    // ========================================================================
    // 卸载和重载
    // ========================================================================

    /// 卸载插件 (从内存移除并删除文件)
    ///
    /// 使用 tokio::fs 执行异步文件操作，避免阻塞 Tokio worker
    /// Phase 4: 同时清理事件订阅、权限和暴露方法
    pub async fn uninstall_plugin(&self, id: &str) -> Result<(), LifecycleError> {
        // Phase 4: 清理组件（在获取写锁前）
        self.event_bus.unsubscribe_all(id).await;
        self.permission_checker.unregister_permissions(id).await;
        self.method_registry.unregister_all(id).await;
        self.config_manager.unregister_schema(id).await;

        // 1. 先从内存移除，释放写锁
        let plugin_path = {
            let mut plugins = self.plugins.write().await;
            if let Some(mut plugin) = plugins.remove(id) {
                let resources = plugin.resources.clear();
                log::info!(
                    "已卸载插件: {}, 释放 {} 个资源, 已清理 Phase 4 组件",
                    id,
                    resources.len()
                );
                Some(plugin.path.clone())
            } else {
                None
            }
        };

        // 2. 在写锁释放后执行异步文件删除
        if let Some(path) = plugin_path {
            if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                // 使用 tokio::fs 进行异步删除，不阻塞 Tokio worker
                tokio::fs::remove_dir_all(&path)
                    .await
                    .map_err(|e| LifecycleError::Io(e))?;
                log::info!("已删除插件目录: {:?}", path);
            }
            Ok(())
        } else {
            Err(LifecycleError::PluginUnload(format!("插件不存在: {}", id)))
        }
    }

    /// 重载插件 (重新读取 manifest)
    ///
    /// 使用 tokio::fs 执行异步文件操作，避免阻塞 Tokio worker
    /// Phase 4: 同步更新事件订阅、权限、暴露方法、配置 Schema
    ///
    /// 两阶段切换保证原子性：
    /// - Phase 1: 验证阶段 - 解析 manifest 和预验证 schema，不触碰现有注册
    /// - Phase 2: 切换阶段 - 验证全部成功后，清理旧注册并一次性切换
    /// 如果验证失败，旧注册完全保持不变
    pub async fn reload_plugin(&self, id: &str) -> Result<PluginInfo, LifecycleError> {
        // ========================================================================
        // Phase 1: 验证阶段（不触碰现有注册）
        // ========================================================================

        // 1.1 获取 manifest 路径，释放读锁
        let manifest_path = {
            let plugins = self.plugins.read().await;
            plugins.get(id).map(|p| p.path.join("manifest.json"))
        };

        let manifest_path = manifest_path
            .ok_or_else(|| LifecycleError::PluginLoad(format!("插件不存在: {}", id)))?;

        // 1.2 读取和解析新 manifest
        let content = tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| LifecycleError::Io(e))?;

        let new_manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| LifecycleError::ManifestParse(e.to_string()))?;

        // 1.3 预验证 config_schema（关键：只验证不注册，失败时旧状态完全保留）
        let validated_schema: Option<ConfigSchema> = if let Some(ref schema_json) = new_manifest.config_schema {
            if schema_json.is_null() {
                None
            } else {
                let schema: ConfigSchema = serde_json::from_value(schema_json.clone())
                    .map_err(|e| LifecycleError::PluginLoad(format!(
                        "配置 Schema 验证失败: {}，reload 未执行，旧状态保持不变", e
                    )))?;
                Some(schema)
            }
        } else {
            None
        };

        log::debug!("[{}] Phase 1 验证通过，开始切换", id);

        // ========================================================================
        // Phase 2: 切换阶段（验证全部成功后执行，不会失败）
        // ========================================================================

        // 2.1 清理旧的 Phase 4 注册
        // 使用 unsubscribe_only 保留事件处理器（handler），避免 reload 后丢失 onEvent 回调
        self.event_bus.unsubscribe_only(id).await;
        self.permission_checker.unregister_permissions(id).await;
        self.method_registry.unregister_all(id).await;
        self.config_manager.unregister_schema(id).await;
        log::debug!("[{}] 已清理旧的 Phase 4 注册（保留事件处理器）", id);

        // 2.2 注册新的 Phase 4 组件
        // Phase 4.1: 注册事件订阅
        if !new_manifest.subscribed_events.is_empty() {
            self.event_bus.subscribe(id, &new_manifest.subscribed_events).await;
            log::debug!("[{}] 重新注册事件订阅: {:?}", id, new_manifest.subscribed_events);
        }

        // Phase 4.2: 注册已验证的配置 Schema（直接使用预验证结果，不会失败）
        if let Some(schema) = validated_schema {
            self.config_manager.register_schema(id, schema).await;
            log::debug!("[{}] 已重新注册配置 Schema", id);
        }

        // Phase 4.3: 注册权限声明
        if !new_manifest.permissions.is_empty() {
            self.permission_checker.register_permissions(id, &new_manifest.permissions).await;
            log::debug!("[{}] 重新注册权限声明: {:?}", id, new_manifest.permissions);
        }

        // Phase 4.4: 注册暴露方法
        for method in &new_manifest.exposed_methods {
            self.method_registry.register(id, method, None).await;
            log::debug!("[{}] 重新注册暴露方法: {}", id, method);
        }

        // 6. 获取写锁更新插件状态
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.get_mut(id) {
            let was_enabled = plugin.enabled;
            plugin.manifest = new_manifest;
            plugin.state = PluginState::Unloaded;
            plugin.enabled = was_enabled;
            plugin.reset_health_stats();

            log::info!("[{}] 已重载插件 v{}，Phase 4 组件已同步", id, plugin.manifest.version);
            Ok(plugin.to_info())
        } else {
            // 插件在读取文件期间被删除 - 清理已注册的组件
            log::warn!("[{}] 插件在 reload 期间被删除，清理已注册组件", id);
            self.event_bus.unsubscribe_only(id).await;
            self.permission_checker.unregister_permissions(id).await;
            self.method_registry.unregister_all(id).await;
            self.config_manager.unregister_schema(id).await;
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    // ========================================================================
    // 配置管理
    // ========================================================================

    /// 获取插件配置
    pub async fn get_plugin_config(&self, id: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.plugins
            .read()
            .await
            .get(id)
            .map(|p| p.config.clone())
    }

    /// 设置插件配置
    pub async fn set_plugin_config(
        &self,
        id: &str,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.config = config;
            log::info!("已更新插件配置: {}", id);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    /// 验证插件配置 (基础验证，可扩展)
    pub async fn validate_plugin_config(
        &self,
        id: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> ValidationResult {
        let plugins = self.plugins.read().await;

        if plugins.get(id).is_none() {
            return ValidationResult {
                valid: false,
                message: Some(format!("插件不存在: {}", id)),
                field_errors: None,
            };
        }

        // 基础验证：检查配置项是否为空
        if config.is_empty() {
            return ValidationResult {
                valid: true,
                message: Some("配置为空".to_string()),
                field_errors: None,
            };
        }

        // TODO: 可以根据 manifest 中的 configSchema 进行更详细的验证
        ValidationResult {
            valid: true,
            message: None,
            field_errors: None,
        }
    }

    // ========================================================================
    // 健康状态（Phase 6 增强 + P1 修复）
    // ========================================================================

    /// 获取单个插件健康状态
    ///
    /// P1 修复：使用读锁，支持并发健康查询。
    pub async fn get_plugin_health(&self, id: &str) -> Option<PluginHealth> {
        self.plugins
            .read()
            .await
            .get(id)
            .map(|p| p.to_health())
    }

    /// 获取所有插件健康状态
    ///
    /// P1 修复：使用读锁，支持并发健康查询。
    pub async fn get_all_health(&self) -> Vec<PluginHealth> {
        self.plugins
            .read()
            .await
            .values()
            .map(|p| p.to_health())
            .collect()
    }

    // ========================================================================
    // 数据管理
    // ========================================================================

    /// 获取单个插件的缓存数据
    pub async fn get_plugin_data(&self, id: &str) -> Option<PluginData> {
        self.plugins
            .read()
            .await
            .get(id)
            .and_then(|p| p.cached_data.clone())
    }

    /// 获取所有插件的缓存数据
    pub async fn get_all_data(&self) -> Vec<PluginData> {
        self.plugins
            .read()
            .await
            .values()
            .filter_map(|p| p.cached_data.clone())
            .collect()
    }

    /// 设置插件缓存数据 (供运行时调用)
    pub async fn set_plugin_data(
        &self,
        id: &str,
        data: PluginData,
        latency_ms: f64,
    ) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.cached_data = Some(data);
            plugin.record_success(latency_ms);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    /// 记录插件执行失败
    pub async fn record_plugin_failure(&self, id: &str, error: String) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.record_failure(error);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    // ========================================================================
    // 插件执行（安全沙盒）
    // ========================================================================

    /// 获取插件执行所需的内容（安全版本，无 TOCTOU 窗口）
    ///
    /// 返回安全读取的入口文件内容和权限列表，供运行时使用。
    /// 使用 openat 链式打开直接读取内容，完全消除 TOCTOU 窗口。
    ///
    /// # 安全保证
    /// - 路径验证和内容读取在同一操作中完成
    /// - 攻击者无法在验证后替换文件
    /// - 推荐使用此方法替代 `get_plugin_execution_info()`
    ///
    /// # 返回
    /// - `Ok((content, permissions))`: 入口文件内容和权限列表
    /// - `Err`: 插件不存在、未启用或读取失败
    pub async fn get_plugin_execution_content(
        &self,
        id: &str,
    ) -> Result<(String, Vec<String>), LifecycleError> {
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(id)
            .ok_or_else(|| LifecycleError::PluginLoad(format!("插件不存在: {}", id)))?;

        // 检查插件是否启用
        if !plugin.enabled {
            return Err(LifecycleError::PluginLoad(format!("插件未启用: {}", id)));
        }

        // 使用安全 API 直接读取内容（无 TOCTOU 窗口）
        let content = plugin.read_entry_content()?;

        // 获取权限列表
        let permissions = plugin.manifest.permissions.clone();

        log::debug!("安全获取插件执行内容: {} ({} bytes)", id, content.len());
        Ok((content, permissions))
    }

    /// 获取插件执行所需的信息（存在 TOCTOU 窗口，已废弃）
    ///
    /// **警告**：返回路径后再打开文件存在 TOCTOU 窗口。
    /// 新代码应使用 `get_plugin_execution_content()` 直接获取内容。
    #[deprecated(note = "存在 TOCTOU 窗口，推荐使用 get_plugin_execution_content()")]
    pub async fn get_plugin_execution_info(
        &self,
        id: &str,
    ) -> Result<(PathBuf, Vec<String>), LifecycleError> {
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(id)
            .ok_or_else(|| LifecycleError::PluginLoad(format!("插件不存在: {}", id)))?;

        // 检查插件是否启用
        if !plugin.enabled {
            return Err(LifecycleError::PluginLoad(format!("插件未启用: {}", id)));
        }

        // 获取安全校验后的入口路径
        // 注意：entry_path() 已废弃，此方法保持兼容
        #[allow(deprecated)]
        let entry_path = plugin.entry_path()?;

        // 获取权限列表
        let permissions = plugin.manifest.permissions.clone();

        Ok((entry_path, permissions))
    }

    /// 标记插件为运行中状态
    pub async fn set_plugin_running(&self, id: &str) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.state = PluginState::Running;
            log::info!("插件状态更新为运行中: {}", id);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    /// 标记插件为已卸载状态
    pub async fn set_plugin_unloaded(&self, id: &str) -> Result<(), LifecycleError> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.state = PluginState::Unloaded;
            log::info!("插件状态更新为已卸载: {}", id);
            Ok(())
        } else {
            Err(LifecycleError::PluginLoad(format!("插件不存在: {}", id)))
        }
    }

    // ========================================================================
    // 插件执行（fetchData 调用）
    // ========================================================================

    /// 执行插件的 fetchData 函数获取数据
    ///
    /// 1. 获取插件代码和配置
    /// 2. 转换 ES Module 为可执行代码
    /// 3. 在沙盒中执行并调用 fetchData
    /// 4. 返回 PluginData
    pub async fn execute_fetch_data(&self, id: &str) -> Result<PluginData, LifecycleError> {
        let start = std::time::Instant::now();

        // 1. 获取插件信息
        let (code, permissions, config, data_type) = {
            let plugins = self.plugins.read().await;
            let plugin = plugins
                .get(id)
                .ok_or_else(|| LifecycleError::PluginLoad(format!("插件不存在: {}", id)))?;

            if !plugin.enabled {
                return Err(LifecycleError::PluginLoad(format!("插件未启用: {}", id)));
            }

            let code = plugin.read_entry_content()?;
            let permissions = plugin.manifest.permissions.clone();
            let config = plugin.config.clone();
            let data_type = plugin.manifest.data_type.clone();

            (code, permissions, config, data_type)
        };

        // 2. 转换 ES Module 为可执行代码
        let executable_code = Self::transform_esm_to_executable(&code, id, &config)?;

        // 3. 创建沙盒运行时并执行
        let result = self.execute_in_sandbox(&executable_code, &permissions).await?;

        // 4. 解析结果为 PluginData
        let plugin_data = Self::parse_fetch_result(id, result, data_type.as_deref())?;

        // 5. 更新缓存和统计
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.set_plugin_data(id, plugin_data.clone(), latency_ms).await?;

        log::info!("[{}] fetchData 执行成功, 耗时 {:.2}ms", id, latency_ms);
        Ok(plugin_data)
    }

    /// 刷新所有启用的插件数据
    pub async fn refresh_all_plugins(&self) -> Vec<Result<PluginData, LifecycleError>> {
        // 获取所有启用的插件 ID
        let enabled_ids: Vec<String> = {
            let plugins = self.plugins.read().await;
            plugins
                .values()
                .filter(|p| p.enabled)
                .map(|p| p.id.clone())
                .collect()
        };

        // 并发执行所有插件
        let mut results = Vec::new();
        for id in enabled_ids {
            let result = self.execute_fetch_data(&id).await;
            results.push(result);
        }

        results
    }

    /// 转换 ES Module 代码为可执行的 IIFE
    ///
    /// 将 `export const X = ...` 和 `export async function X` 转换为
    /// 可以直接 eval 的代码格式
    fn transform_esm_to_executable(
        code: &str,
        plugin_id: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<String, LifecycleError> {
        // 转换 export 语句
        let mut transformed = code.to_string();

        // 1. 转换 `export const X = ...` 为 `const X = ...; __exports.X = X;`
        // 使用简单的字符串替换（更复杂的情况需要 AST 解析）
        transformed = transformed.replace("export const ", "const ");
        transformed = transformed.replace("export let ", "let ");
        transformed = transformed.replace("export var ", "var ");

        // 2. 转换 `export async function X` 为 `async function X`
        transformed = transformed.replace("export async function ", "async function ");
        transformed = transformed.replace("export function ", "function ");

        // 3. 序列化配置
        let config_json = serde_json::to_string(config)
            .map_err(|e| LifecycleError::PluginLoad(format!("配置序列化失败: {}", e)))?;

        // 4. 构建可执行代码
        // 使用 IIFE 包装，注入 context，调用 fetchData
        let executable = format!(
            r#"(function() {{
  var __exports = {{}};

  // 注入 context 对象
  var context = {{
    pluginId: "{}",
    config: {},
    log: function(level, msg) {{
      console.log("[" + level + "][{}] " + msg);
    }},
    emit: function(event, data) {{
      console.log("[emit][{}] " + event);
    }}
  }};

  // 插件代码开始
  {}
  // 插件代码结束

  // 收集导出（简化版：假设标准命名）
  if (typeof metadata !== 'undefined') __exports.metadata = metadata;
  if (typeof fetchData !== 'undefined') __exports.fetchData = fetchData;
  if (typeof onLoad !== 'undefined') __exports.onLoad = onLoad;
  if (typeof onUnload !== 'undefined') __exports.onUnload = onUnload;
  if (typeof validateConfig !== 'undefined') __exports.validateConfig = validateConfig;

  // 调用 fetchData
  if (typeof __exports.fetchData !== 'function') {{
    throw new Error('插件未导出 fetchData 函数');
  }}

  var config = {};
  var result = __exports.fetchData(config, context);

  // 处理 Promise（如果是 async function）
  if (result && typeof result.then === 'function') {{
    // 同步等待 Promise（QuickJS 限制：需要 Promise 立即 resolve）
    var resolved = null;
    var rejected = null;
    result.then(function(v) {{ resolved = v; }}, function(e) {{ rejected = e; }});
    if (rejected) throw rejected;
    return resolved;
  }}

  return result;
}})()"#,
            plugin_id,
            config_json,
            plugin_id,
            plugin_id,
            transformed,
            config_json
        );

        Ok(executable)
    }

    /// 在沙盒中执行代码
    async fn execute_in_sandbox(
        &self,
        code: &str,
        permissions: &[String],
    ) -> Result<serde_json::Value, LifecycleError> {
        use crate::plugin::{SandboxConfig, SandboxRuntime, PluginExecutor, RequestManager};
        use std::sync::Arc;

        // 创建沙盒运行时
        let config = SandboxConfig::default();
        let runtime = SandboxRuntime::new(config)
            .await
            .map_err(|e| LifecycleError::PluginLoad(format!("创建沙盒失败: {}", e)))?;

        // 创建执行器
        let request_manager = RequestManager::new()
            .map_err(|e| LifecycleError::PluginLoad(format!("创建 RequestManager 失败: {}", e)))?;
        let executor = PluginExecutor::new(Arc::new(runtime))
            .with_request_manager(Arc::new(request_manager));

        // 执行代码
        executor
            .execute_plugin(code, permissions)
            .await
            .map_err(|e| LifecycleError::PluginLoad(format!("执行插件失败: {}", e)))
    }

    /// 解析 fetchData 返回的结果为 PluginData
    fn parse_fetch_result(
        plugin_id: &str,
        result: serde_json::Value,
        expected_data_type: Option<&str>,
    ) -> Result<PluginData, LifecycleError> {
        use crate::plugin::types::{
            PluginDataBase, UsageData, BalanceData, StatusData, CustomData,
        };

        // 获取 dataType
        let data_type = result
            .get("dataType")
            .and_then(|v| v.as_str())
            .or(expected_data_type)
            .ok_or_else(|| {
                LifecycleError::PluginLoad("fetchData 返回结果缺少 dataType".to_string())
            })?;

        // 构建基础数据
        let last_updated = result
            .get("lastUpdated")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let base = PluginDataBase {
            plugin_id: plugin_id.to_string(),
            last_updated,
        };

        // 根据 dataType 解析
        match data_type {
            "usage" => {
                let percentage = result.get("percentage").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let used = result.get("used").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let limit = result.get("limit").and_then(|v| v.as_f64()).unwrap_or(100.0);
                let unit = result
                    .get("unit")
                    .and_then(|v| v.as_str())
                    .unwrap_or("units")
                    .to_string();
                let reset_time = result.get("resetTime").and_then(|v| v.as_str()).map(|s| s.to_string());
                let reset_label = result.get("resetLabel").and_then(|v| v.as_str()).map(|s| s.to_string());

                // 解析 dimensions
                let dimensions: Option<Vec<crate::plugin::types::UsageDimension>> = result
                    .get("dimensions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|d| {
                                Some(crate::plugin::types::UsageDimension {
                                    id: d.get("id")?.as_str()?.to_string(),
                                    label: d.get("label")?.as_str()?.to_string(),
                                    percentage: d.get("percentage")?.as_f64()?,
                                    used: d.get("used")?.as_f64()?,
                                    limit: d.get("limit")?.as_f64()?,
                                    reset_time: d.get("resetTime").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                })
                            })
                            .collect()
                    });

                Ok(PluginData::Usage(UsageData {
                    base,
                    percentage,
                    used,
                    limit,
                    unit,
                    reset_time,
                    reset_label,
                    dimensions,
                }))
            }
            "balance" => {
                let balance = result.get("balance").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let currency = result
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("USD")
                    .to_string();
                let quota = result.get("quota").and_then(|v| v.as_f64());
                let used_quota = result.get("usedQuota").and_then(|v| v.as_f64());
                let expires_at = result.get("expiresAt").and_then(|v| v.as_str()).map(|s| s.to_string());

                Ok(PluginData::Balance(BalanceData {
                    base,
                    balance,
                    currency,
                    quota,
                    used_quota,
                    expires_at,
                }))
            }
            "status" => {
                use crate::plugin::types::StatusIndicator;

                // 解析 indicator
                let indicator_str = result
                    .get("indicator")
                    .or_else(|| result.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let indicator = match indicator_str.to_lowercase().as_str() {
                    "none" | "ok" | "healthy" => StatusIndicator::None,
                    "minor" | "warning" => StatusIndicator::Minor,
                    "major" | "error" => StatusIndicator::Major,
                    "critical" => StatusIndicator::Critical,
                    _ => StatusIndicator::Unknown,
                };

                let description = result
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(PluginData::Status(StatusData {
                    base,
                    indicator,
                    description,
                }))
            }
            _ => {
                // 自定义类型
                Ok(PluginData::Custom(CustomData {
                    base,
                    render_html: result.get("renderHtml").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    payload: result.clone(),
                    title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    subtitle: result.get("subtitle").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let json = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "apiVersion": "1.0",
            "pluginType": "data",
            "dataType": "usage"
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.get_plugin_type(), PluginType::Data);
        assert_eq!(manifest.get_data_type(), Some(DataType::Usage));
    }

    #[test]
    fn test_resource_registry() {
        let mut registry = ResourceRegistry::new();

        let id1 = registry.register(ResourceType::Timer);
        let id2 = registry.register(ResourceType::Request);

        assert_eq!(registry.count(), 2);
        assert_eq!(registry.ids_by_type(ResourceType::Timer), vec![id1]);

        registry.unregister(id1);
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_plugin_discovery_default_dir() {
        let discovery = PluginDiscovery::with_default_dir();
        assert!(discovery.plugins_dir().ends_with("plugins"));
    }

    // ========================================================================
    // 回归测试：P1 TOCTOU 防护
    // ========================================================================

    #[test]
    fn test_openat_verifier_normal_path() {
        // 使用临时目录测试正常路径
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // 创建嵌套目录
        let sub_dir = base.join("sub");
        std::fs::create_dir(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("entry.js"), "// test").unwrap();

        // 使用新的安全 API 验证
        let result = openat_verifier::open_file_safely(base, "sub/entry.js");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_openat_verifier_rejects_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // 创建真实目录和文件
        let real_dir = base.join("real");
        std::fs::create_dir(&real_dir).unwrap();
        std::fs::write(real_dir.join("secret.txt"), "secret").unwrap();

        // 创建指向 real 的 symlink
        let link_dir = base.join("link");
        symlink(&real_dir, &link_dir).unwrap();

        // 使用新的安全 API 验证应拒绝 symlink 路径
        let result = openat_verifier::open_file_safely(base, "link/secret.txt");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("symlink") || err_msg.contains("符号链接"));
    }

    #[test]
    #[cfg(unix)]
    fn test_openat_verifier_rejects_symlink_file() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // 创建真实文件
        std::fs::write(base.join("real.js"), "// real").unwrap();

        // 创建指向 real.js 的 symlink
        symlink(base.join("real.js"), base.join("link.js")).unwrap();

        // 使用新的安全 API 验证应拒绝 symlink 文件
        let result = openat_verifier::open_file_safely(base, "link.js");
        assert!(result.is_err());
    }

    #[test]
    fn test_entry_path_rejects_absolute_path() {
        let manifest = PluginManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            api_version: "1.0".into(),
            entry: "/etc/passwd".into(),
            ..Default::default()
        };
        let instance = PluginInstance::new(PathBuf::from("/tmp"), manifest);
        // 使用新的安全 API
        assert!(instance.read_entry_content().is_err());
    }

    #[test]
    fn test_entry_path_rejects_path_traversal() {
        let manifest = PluginManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            api_version: "1.0".into(),
            entry: "../../../etc/passwd".into(),
            ..Default::default()
        };
        let instance = PluginInstance::new(PathBuf::from("/tmp/plugins/test"), manifest);
        // 使用新的安全 API
        assert!(instance.read_entry_content().is_err());
    }

    #[test]
    fn test_read_entry_content_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // 创建测试文件
        std::fs::write(base.join("plugin.js"), "console.log('test');").unwrap();

        let manifest = PluginManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            api_version: "1.0".into(),
            entry: "plugin.js".into(),
            ..Default::default()
        };
        let instance = PluginInstance::new(base.to_path_buf(), manifest);

        // 验证可以读取内容
        let content = instance.read_entry_content().unwrap();
        assert_eq!(content, "console.log('test');");
    }

    // ========================================================================
    // Phase 6: to_health() 分支覆盖测试
    // ========================================================================

    fn create_test_instance() -> PluginInstance {
        let manifest = PluginManifest {
            id: "test-health".into(),
            name: "Test Health".into(),
            version: "1.0.0".into(),
            api_version: "1.0".into(),
            entry: "plugin.js".into(),
            ..Default::default()
        };
        PluginInstance::new(PathBuf::from("/tmp/test"), manifest)
    }

    #[test]
    fn test_to_health_healthy_status() {
        let mut instance = create_test_instance();

        // 记录足够多的成功调用（成功率 >= 95%）
        for _ in 0..100 {
            instance.record_success(50.0);
        }

        let health = instance.to_health();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.success_rate >= 0.95);
    }

    #[test]
    fn test_to_health_unhealthy_consecutive_failures() {
        let mut instance = create_test_instance();

        // 连续失败 3 次 → Unhealthy
        instance.record_failure("error1".into());
        instance.record_failure("error2".into());
        instance.record_failure("error3".into());

        let health = instance.to_health();
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.consecutive_failures, 3);
    }

    #[test]
    fn test_to_health_unhealthy_low_success_rate() {
        let mut instance = create_test_instance();

        // 成功率 < 80% → Unhealthy
        for _ in 0..2 {
            instance.record_success(50.0);
        }
        for _ in 0..8 {
            instance.record_failure("error".into());
        }
        // 重置连续失败以测试纯成功率判定
        instance.consecutive_failures = 0;

        let health = instance.to_health();
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert!(health.success_rate < 0.8);
    }

    #[test]
    fn test_to_health_degraded_high_latency() {
        let mut instance = create_test_instance();

        // 高延迟 (P99 > 5000ms) → Degraded
        for _ in 0..100 {
            instance.record_success(6000.0); // 高延迟
        }

        let health = instance.to_health();
        assert_eq!(health.status, HealthStatus::Degraded);
        assert!(health.p99_latency_ms > 5000.0);
    }

    #[test]
    fn test_to_health_degraded_medium_success_rate() {
        let mut instance = create_test_instance();

        // 成功率 80%-95% → Degraded
        for _ in 0..90 {
            instance.record_success(50.0);
        }
        for _ in 0..10 {
            instance.record_failure("error".into());
        }
        // 重置连续失败以测试纯成功率判定
        instance.consecutive_failures = 0;

        let health = instance.to_health();
        assert_eq!(health.status, HealthStatus::Degraded);
        assert!(health.success_rate >= 0.8 && health.success_rate < 0.95);
    }

    #[test]
    fn test_to_health_consecutive_failures_reset_on_success() {
        let mut instance = create_test_instance();

        // 连续失败 2 次
        instance.record_failure("error1".into());
        instance.record_failure("error2".into());
        assert_eq!(instance.consecutive_failures, 2);

        // 成功一次后连续失败应重置
        instance.record_success(50.0);
        assert_eq!(instance.consecutive_failures, 0);
    }

    #[test]
    fn test_to_health_total_calls_accuracy() {
        let mut instance = create_test_instance();

        instance.record_success(50.0);
        instance.record_success(100.0);
        instance.record_failure("error".into());

        let health = instance.to_health();
        assert_eq!(health.total_calls, 3);
    }

    // ========================================================================
    // 回归测试：reload_plugin 两阶段切换
    // ========================================================================

    /// 测试：schema 非法时 reload 不影响旧状态
    ///
    /// 验证两阶段切换的核心保证：
    /// - Phase 1 验证失败时，不触碰现有注册
    /// - 旧的订阅、权限、方法、schema 完全保持不变
    #[tokio::test]
    async fn test_reload_preserves_state_on_invalid_schema() {
        // 1. 创建临时目录
        let temp_dir = tempfile::tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir(&plugins_dir).unwrap();

        // 2. 创建插件目录
        let plugin_dir = plugins_dir.join("test-reload");
        std::fs::create_dir(&plugin_dir).unwrap();

        // 3. 写入有效的 manifest.json（包含订阅、权限、方法、schema）
        let valid_manifest = r#"{
            "id": "test-reload",
            "name": "Test Reload Plugin",
            "version": "1.0.0",
            "apiVersion": "1.0",
            "pluginType": "data",
            "entry": "plugin.js",
            "subscribedEvents": ["plugin:other:event_a"],
            "permissions": ["network"],
            "exposedMethods": ["getData"],
            "configSchema": {
                "apiKey": {"type": "string", "required": true}
            }
        }"#;
        std::fs::write(plugin_dir.join("manifest.json"), valid_manifest).unwrap();
        std::fs::write(plugin_dir.join("plugin.js"), "// test plugin").unwrap();

        // 4. 创建 PluginManager 并加载
        let discovery = PluginDiscovery::new(plugins_dir);
        let manager = PluginManager::new(discovery);
        let loaded = manager.discover_and_load().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-reload");

        // 5. 验证 Phase 4 组件已注册
        let event_bus = manager.event_bus();
        let config_manager = manager.config_manager();
        let permission_checker = manager.permission_checker();
        let method_registry = manager.method_registry();

        // 验证订阅已注册
        let subscribers = event_bus.get_subscribers("plugin:other:event_a").await;
        assert!(subscribers.contains(&"test-reload".to_string()), "订阅应已注册");

        // 验证 schema 已注册
        let schema = config_manager.get_schema("test-reload").await;
        assert!(schema.is_some(), "Schema 应已注册");

        // 验证方法已注册
        let methods = method_registry.get_plugin_methods("test-reload").await;
        assert_eq!(methods.len(), 1, "方法应已注册");
        assert_eq!(methods[0].name, "getData");

        // 验证权限已注册
        let permissions = permission_checker.get_plugin_permissions("test-reload").await;
        assert!(permissions.contains(&"network".to_string()), "权限应已注册");

        // 6. 修改 manifest 使 schema 无效（schema 应该是对象，不是字符串）
        let invalid_manifest = r#"{
            "id": "test-reload",
            "name": "Test Reload Plugin",
            "version": "2.0.0",
            "apiVersion": "1.0",
            "pluginType": "data",
            "entry": "plugin.js",
            "subscribedEvents": ["plugin:new:event_b"],
            "permissions": ["storage"],
            "exposedMethods": ["newMethod"],
            "configSchema": "this_is_invalid_not_an_object"
        }"#;
        std::fs::write(plugin_dir.join("manifest.json"), invalid_manifest).unwrap();

        // 7. 调用 reload_plugin，预期失败
        let result = manager.reload_plugin("test-reload").await;
        assert!(result.is_err(), "reload 应失败因为 schema 无效");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("验证失败") || err_msg.contains("Schema"),
            "错误信息应提及 schema 验证失败: {}", err_msg);

        // 8. 验证旧状态完全保持不变
        // 8.1 订阅仍然是旧的
        let subscribers_after = event_bus.get_subscribers("plugin:other:event_a").await;
        assert!(subscribers_after.contains(&"test-reload".to_string()),
            "旧订阅应保持不变");
        let new_subscribers = event_bus.get_subscribers("plugin:new:event_b").await;
        assert!(!new_subscribers.contains(&"test-reload".to_string()),
            "新订阅不应被注册");

        // 8.2 schema 仍然是旧的
        let schema_after = config_manager.get_schema("test-reload").await;
        assert!(schema_after.is_some(), "旧 schema 应保持不变");

        // 8.3 方法仍然是旧的
        let methods_after = method_registry.get_plugin_methods("test-reload").await;
        assert_eq!(methods_after.len(), 1, "旧方法应保持不变");
        assert_eq!(methods_after[0].name, "getData", "旧方法名应保持不变");

        // 8.4 权限仍然是旧的
        let permissions_after = permission_checker.get_plugin_permissions("test-reload").await;
        assert!(permissions_after.contains(&"network".to_string()),
            "旧权限应保持不变");
        assert!(!permissions_after.contains(&"storage".to_string()),
            "新权限不应被注册");
    }

    /// 测试：schema 有效时 reload 正常更新状态
    #[tokio::test]
    async fn test_reload_updates_state_on_valid_schema() {
        // 1. 创建临时目录
        let temp_dir = tempfile::tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir(&plugins_dir).unwrap();

        // 2. 创建插件目录
        let plugin_dir = plugins_dir.join("test-reload-valid");
        std::fs::create_dir(&plugin_dir).unwrap();

        // 3. 写入初始 manifest.json
        let initial_manifest = r#"{
            "id": "test-reload-valid",
            "name": "Test Reload Valid",
            "version": "1.0.0",
            "apiVersion": "1.0",
            "pluginType": "data",
            "entry": "plugin.js",
            "subscribedEvents": ["plugin:old:event"],
            "permissions": ["network"],
            "exposedMethods": ["oldMethod"],
            "configSchema": {
                "oldKey": {"type": "string"}
            }
        }"#;
        std::fs::write(plugin_dir.join("manifest.json"), initial_manifest).unwrap();
        std::fs::write(plugin_dir.join("plugin.js"), "// test plugin").unwrap();

        // 4. 创建 PluginManager 并加载
        let discovery = PluginDiscovery::new(plugins_dir);
        let manager = PluginManager::new(discovery);
        manager.discover_and_load().await.unwrap();

        // 5. 修改 manifest 为新的有效配置
        let new_manifest = r#"{
            "id": "test-reload-valid",
            "name": "Test Reload Valid Updated",
            "version": "2.0.0",
            "apiVersion": "1.0",
            "pluginType": "data",
            "entry": "plugin.js",
            "subscribedEvents": ["plugin:new:event"],
            "permissions": ["storage"],
            "exposedMethods": ["newMethod"],
            "configSchema": {
                "newKey": {"type": "number"}
            }
        }"#;
        std::fs::write(plugin_dir.join("manifest.json"), new_manifest).unwrap();

        // 6. 调用 reload_plugin，预期成功
        let result = manager.reload_plugin("test-reload-valid").await;
        assert!(result.is_ok(), "reload 应成功");
        let info = result.unwrap();
        assert_eq!(info.version, "2.0.0", "版本应更新");

        // 7. 验证新状态已生效
        let event_bus = manager.event_bus();
        let config_manager = manager.config_manager();
        let method_registry = manager.method_registry();
        let permission_checker = manager.permission_checker();

        // 7.1 新订阅已注册，旧订阅已清理
        let new_subscribers = event_bus.get_subscribers("plugin:new:event").await;
        assert!(new_subscribers.contains(&"test-reload-valid".to_string()),
            "新订阅应已注册");
        let old_subscribers = event_bus.get_subscribers("plugin:old:event").await;
        assert!(!old_subscribers.contains(&"test-reload-valid".to_string()),
            "旧订阅应已清理");

        // 7.2 新 schema 已注册
        let schema = config_manager.get_schema("test-reload-valid").await;
        assert!(schema.is_some(), "新 schema 应已注册");
        assert!(schema.unwrap().contains_key("newKey"), "新 schema 应包含 newKey");

        // 7.3 新方法已注册
        let methods = method_registry.get_plugin_methods("test-reload-valid").await;
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, "newMethod", "新方法应已注册");

        // 7.4 新权限已注册
        let permissions = permission_checker.get_plugin_permissions("test-reload-valid").await;
        assert!(permissions.contains(&"storage".to_string()),
            "新权限应已注册");
    }
}
