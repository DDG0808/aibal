// Phase 5A.2: 插件安装器
// 实现从 URL 或 registry 安装插件的完整流程

use crate::plugin::types::{AppError, PluginInfo, Result as IpcResult};
use crate::plugin::PluginManager;
use crate::security::{verify_manifest_signature, verify_manifest_files, SecureExtractor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// ============================================================================
// 插件状态保存（用于更新时保留配置）
// ============================================================================

/// 保存的插件状态（配置和启用状态）
#[derive(Debug, Clone)]
struct SavedPluginState {
    /// 插件配置
    config: HashMap<String, serde_json::Value>,
    /// 是否启用
    enabled: bool,
}

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("下载失败: {0}")]
    Download(String),

    #[error("解压失败: {0}")]
    Extract(String),

    #[error("manifest 解析失败: {0}")]
    ManifestParse(String),

    #[error("签名验证失败: {0}")]
    SignatureInvalid(String),

    #[error("完整性验证失败: {0}")]
    IntegrityFailed(String),

    #[error("安装失败: {0}")]
    Install(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("无效的 source: {0}")]
    InvalidSource(String),
}

impl From<InstallError> for AppError {
    fn from(e: InstallError) -> Self {
        let code = match &e {
            InstallError::Download(_) => "DOWNLOAD_FAILED",
            InstallError::Extract(_) => "EXTRACT_FAILED",
            InstallError::ManifestParse(_) => "MANIFEST_PARSE_FAILED",
            InstallError::SignatureInvalid(_) => "SIGNATURE_INVALID",
            InstallError::IntegrityFailed(_) => "INTEGRITY_FAILED",
            InstallError::Install(_) => "INSTALL_FAILED",
            InstallError::Io(_) => "IO_ERROR",
            InstallError::InvalidSource(_) => "INVALID_SOURCE",
        };
        AppError::new(code, e.to_string())
    }
}

// ============================================================================
// 插件安装器
// ============================================================================

pub struct PluginInstaller {
    plugin_manager: Arc<PluginManager>,
    http_client: reqwest::Client,
}

impl PluginInstaller {
    pub fn new(plugin_manager: Arc<PluginManager>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            plugin_manager,
            http_client,
        }
    }

    /// 安装插件
    ///
    /// # 参数
    /// - `source`: 插件源，支持两种格式：
    ///   - URL: `https://example.com/plugin.zip`
    ///   - Registry: `registry://plugin-id`
    /// - `skip_signature`: 是否跳过签名验证
    /// - `registry_url`: 市场 registry.json URL（用于 registry:// 协议）
    pub async fn install(
        &self,
        source: &str,
        skip_signature: bool,
        registry_url: Option<&str>,
    ) -> Result<PluginInfo, InstallError> {
        log::info!("开始安装插件: source={}, skip_signature={}", source, skip_signature);

        // 1. 解析 source，获取下载 URL
        let download_url = self.resolve_source(source, registry_url).await?;
        log::debug!("解析后的下载 URL: {}", download_url);

        // 2. 创建临时目录
        let temp_dir = TempDir::new().map_err(|e| InstallError::Io(e.into()))?;
        let zip_path = temp_dir.path().join("plugin.zip");
        let extract_dir = temp_dir.path().join("extracted");

        // 3. 下载 ZIP 文件
        self.download(&download_url, &zip_path).await?;
        log::debug!("下载完成: {:?}", zip_path);

        // 4. 安全解压
        let extractor = SecureExtractor::new();
        extractor
            .extract(&zip_path, &extract_dir)
            .map_err(|e| InstallError::Extract(e.to_string()))?;
        log::debug!("解压完成: {:?}", extract_dir);

        // 5. 解析 manifest.json
        let manifest_path = extract_dir.join("manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| InstallError::ManifestParse(format!("读取 manifest 失败: {}", e)))?;

        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
            .map_err(|e| InstallError::ManifestParse(format!("JSON 解析失败: {}", e)))?;

        // 6. 获取插件 ID
        let plugin_id = manifest
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InstallError::ManifestParse("manifest 缺少 id 字段".to_string()))?
            .to_string();

        log::info!("解析 manifest 成功: id={}", plugin_id);

        // 7. 签名验证（如果不跳过）
        if !skip_signature {
            verify_manifest_signature(&manifest)
                .map_err(|e| InstallError::SignatureInvalid(e.to_string()))?;
            log::info!("签名验证通过: {}", plugin_id);
        } else {
            log::warn!("跳过签名验证: {}", plugin_id);
        }

        // 8. 完整性验证
        verify_manifest_files(&manifest, &extract_dir)
            .map_err(|e| InstallError::IntegrityFailed(e.to_string()))?;
        log::info!("完整性验证通过: {}", plugin_id);

        // 9. 保存旧插件的配置和启用状态（用于更新时保留）
        let saved_state = self.save_plugin_state(&plugin_id).await;
        if saved_state.is_some() {
            log::info!("已保存插件 {} 的配置和启用状态", plugin_id);
        }

        // 10. 移动到插件目录
        let target_dir = self.plugin_manager.plugins_dir().join(&plugin_id);

        // 如果目录已存在，先备份
        if target_dir.exists() {
            let backup_dir = self.plugin_manager.plugins_dir().join(format!("{}.backup", plugin_id));
            if backup_dir.exists() {
                fs::remove_dir_all(&backup_dir).await?;
            }
            fs::rename(&target_dir, &backup_dir).await?;
            log::info!("已备份旧版本: {:?}", backup_dir);
        }

        // 移动解压目录到插件目录
        self.move_dir(&extract_dir, &target_dir).await?;
        log::info!("插件已安装到: {:?}", target_dir);

        // 11. 重新加载插件
        let plugins = self
            .plugin_manager
            .discover_and_load()
            .await
            .map_err(|e| InstallError::Install(format!("加载插件失败: {}", e)))?;

        // 12. 恢复配置和启用状态
        if let Some(state) = saved_state {
            self.restore_plugin_state(&plugin_id, state).await;
            log::info!("已恢复插件 {} 的配置和启用状态", plugin_id);
        }

        // 13. 查找并返回新安装的插件信息
        let plugin_info = plugins
            .into_iter()
            .find(|p| p.id == plugin_id)
            .ok_or_else(|| InstallError::Install(format!("插件加载后未找到: {}", plugin_id)))?;

        log::info!("插件安装成功: {} v{}", plugin_info.id, plugin_info.version);
        Ok(plugin_info)
    }

    /// 保存插件的配置和启用状态
    async fn save_plugin_state(&self, plugin_id: &str) -> Option<SavedPluginState> {
        // 获取插件列表，检查插件是否存在
        let plugins = self.plugin_manager.list_plugins().await;
        let plugin = plugins.iter().find(|p| p.id == plugin_id)?;

        // 获取配置
        let config = self.plugin_manager.get_plugin_config(plugin_id).await.unwrap_or_default();

        Some(SavedPluginState {
            config,
            enabled: plugin.enabled,
        })
    }

    /// 恢复插件的配置和启用状态
    async fn restore_plugin_state(&self, plugin_id: &str, state: SavedPluginState) {
        // 恢复配置
        if !state.config.is_empty() {
            if let Err(e) = self.plugin_manager.set_plugin_config(plugin_id, state.config).await {
                log::warn!("恢复插件 {} 配置失败: {}", plugin_id, e);
            }
        }

        // 恢复启用状态
        if state.enabled {
            if let Err(e) = self.plugin_manager.enable_plugin(plugin_id).await {
                log::warn!("恢复插件 {} 启用状态失败: {}", plugin_id, e);
            }
        }
    }

    /// 解析 source，返回下载 URL
    async fn resolve_source(
        &self,
        source: &str,
        registry_url: Option<&str>,
    ) -> Result<String, InstallError> {
        if source.starts_with("registry://") {
            // registry://plugin-id 格式
            let plugin_id = source
                .strip_prefix("registry://")
                .ok_or_else(|| InstallError::InvalidSource("无效的 registry 协议".to_string()))?;

            let registry_url = registry_url.ok_or_else(|| {
                InstallError::InvalidSource("使用 registry:// 协议需要提供 registry_url".to_string())
            })?;

            // 获取 registry.json
            let registry = self.fetch_registry(registry_url).await?;

            // 查找插件的 downloadUrl
            let plugins = registry
                .get("plugins")
                .and_then(|v| v.as_array())
                .ok_or_else(|| InstallError::InvalidSource("registry 格式无效".to_string()))?;

            for plugin in plugins {
                if plugin.get("id").and_then(|v| v.as_str()) == Some(plugin_id) {
                    if let Some(url) = plugin.get("downloadUrl").and_then(|v| v.as_str()) {
                        return Ok(url.to_string());
                    }
                }
            }

            Err(InstallError::InvalidSource(format!(
                "在 registry 中未找到插件: {}",
                plugin_id
            )))
        } else if source.starts_with("http://") || source.starts_with("https://") {
            // 直接 URL
            Ok(source.to_string())
        } else {
            Err(InstallError::InvalidSource(format!(
                "不支持的 source 格式: {}",
                source
            )))
        }
    }

    /// 获取 registry.json
    async fn fetch_registry(&self, url: &str) -> Result<serde_json::Value, InstallError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| InstallError::Download(format!("获取 registry 失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(InstallError::Download(format!(
                "获取 registry 失败: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| InstallError::Download(format!("解析 registry 失败: {}", e)))
    }

    /// 下载文件
    async fn download(&self, url: &str, target: &Path) -> Result<(), InstallError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| InstallError::Download(format!("HTTP 请求失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(InstallError::Download(format!(
                "下载失败: HTTP {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| InstallError::Download(format!("读取响应失败: {}", e)))?;

        let mut file = fs::File::create(target).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;

        Ok(())
    }

    /// 移动目录（跨文件系统兼容）
    async fn move_dir(&self, src: &Path, dst: &Path) -> Result<(), InstallError> {
        // 尝试直接重命名
        if fs::rename(src, dst).await.is_ok() {
            return Ok(());
        }

        // 跨文件系统时需要复制
        self.copy_dir_recursive(src, dst).await?;
        fs::remove_dir_all(src).await?;
        Ok(())
    }

    /// 递归复制目录
    async fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<(), InstallError> {
        fs::create_dir_all(dst).await?;

        let mut entries = fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if entry.file_type().await?.is_dir() {
                Box::pin(self.copy_dir_recursive(&src_path, &dst_path)).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 安装插件（便捷函数）
pub async fn install_plugin(
    plugin_manager: Arc<PluginManager>,
    source: &str,
    skip_signature: bool,
    registry_url: Option<&str>,
) -> Result<PluginInfo, InstallError> {
    let installer = PluginInstaller::new(plugin_manager);
    installer.install(source, skip_signature, registry_url).await
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_error_to_app_error() {
        let error = InstallError::Download("test error".to_string());
        let app_error: AppError = error.into();
        assert_eq!(app_error.code, "DOWNLOAD_FAILED");
    }

    #[test]
    fn test_invalid_source_error() {
        let error = InstallError::InvalidSource("bad source".to_string());
        let app_error: AppError = error.into();
        assert_eq!(app_error.code, "INVALID_SOURCE");
    }
}
