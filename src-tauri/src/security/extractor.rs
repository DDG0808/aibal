// Phase 5A.3: 安全解压器
// 提供安全的 ZIP 解压功能，防止路径穿越、符号链接等攻击

use crate::security::{
    Result, SecurityError, ALLOWED_EXTENSIONS, BACKUP_VERSIONS, MAX_ENTRIES, MAX_FILE_SIZE,
    MAX_TOTAL_SIZE,
};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zip::ZipArchive;

// ============================================================================
// 安全解压器
// ============================================================================

/// 安全 ZIP 解压器
///
/// 提供以下安全保护:
/// - 路径穿越检测 (拒绝包含 ".." 的路径)
/// - 符号链接拒绝
/// - 单文件大小限制 (10MB)
/// - 总大小限制 (50MB)
/// - 文件类型白名单 (.js/.json/.png/.svg)
/// - 原子替换 (临时目录 → 重命名)
/// - 备份与回滚
pub struct SecureExtractor {
    /// 单文件大小限制
    max_file_size: u64,
    /// 总大小限制
    max_total_size: u64,
    /// 最大条目数限制
    max_entries: usize,
    /// 允许的扩展名
    allowed_extensions: Vec<String>,
    /// 备份保留版本数
    backup_versions: usize,
}

impl SecureExtractor {
    /// 创建默认配置的解压器
    pub fn new() -> Self {
        Self {
            max_file_size: MAX_FILE_SIZE,
            max_total_size: MAX_TOTAL_SIZE,
            max_entries: MAX_ENTRIES,
            allowed_extensions: ALLOWED_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            backup_versions: BACKUP_VERSIONS,
        }
    }

    /// 自定义配置
    pub fn with_limits(mut self, max_file_size: u64, max_total_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self.max_total_size = max_total_size;
        self
    }

    /// 自定义允许的扩展名
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = extensions;
        self
    }

    /// 自定义备份版本数
    pub fn with_backup_versions(mut self, versions: usize) -> Self {
        self.backup_versions = versions;
        self
    }

    /// 安全解压 ZIP 文件到目标目录
    ///
    /// # 参数
    /// - `zip_path`: ZIP 文件路径
    /// - `target_dir`: 目标目录
    ///
    /// # 返回
    /// - `Ok(())`: 解压成功
    /// - `Err(SecurityError)`: 解压失败
    ///
    /// # 安全检查
    /// 1. 路径穿越检测
    /// 2. 符号链接拒绝
    /// 3. 文件大小限制
    /// 4. 文件类型白名单
    /// 5. 原子替换
    pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        zip_path: P,
        target_dir: Q,
    ) -> Result<()> {
        let zip_path = zip_path.as_ref();
        let target_dir = target_dir.as_ref();

        // 1. 打开 ZIP 文件
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        // 2. 预检查：验证所有文件
        self.pre_validate(&mut archive)?;

        // 3. 创建临时目录
        let temp_dir = TempDir::new()?;

        // 4. 解压到临时目录
        self.extract_to_temp(&mut archive, temp_dir.path())?;

        // 5. 备份现有目录
        if target_dir.exists() {
            self.create_backup(target_dir)?;
        }

        // 6. 原子替换
        self.atomic_replace(temp_dir.path(), target_dir)?;

        log::info!("安全解压完成: {:?} -> {:?}", zip_path, target_dir);
        Ok(())
    }

    /// 预验证 ZIP 内容
    fn pre_validate(&self, archive: &mut ZipArchive<File>) -> Result<()> {
        // 检查条目数限制 (防止 ZIP 炸弹)
        let entry_count = archive.len();
        if entry_count > self.max_entries {
            return Err(SecurityError::TooManyEntries {
                count: entry_count,
                limit: self.max_entries,
            });
        }

        let mut total_size: u64 = 0;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;

            // 使用 enclosed_name() 安全获取路径 (处理所有平台的路径穿越情况)
            let safe_name = file.enclosed_name().ok_or_else(|| {
                SecurityError::PathTraversal {
                    path: file.name().to_string(),
                }
            })?;
            let name = safe_name.to_string_lossy().to_string();

            // 检查符号链接
            if file.is_symlink() {
                return Err(SecurityError::SymlinkRejected { path: name });
            }

            // 跳过目录
            if file.is_dir() {
                continue;
            }

            // 检查文件类型
            if !self.is_allowed_extension(&name) {
                let ext = Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("(none)")
                    .to_string();
                return Err(SecurityError::FileTypeNotAllowed {
                    file: name,
                    extension: ext,
                });
            }

            // 检查单文件大小
            let size = file.size();
            if size > self.max_file_size {
                return Err(SecurityError::FileTooLarge {
                    file: name,
                    size,
                    limit: self.max_file_size,
                });
            }

            total_size += size;
        }

        // 检查总大小
        if total_size > self.max_total_size {
            return Err(SecurityError::TotalSizeTooLarge {
                total: total_size,
                limit: self.max_total_size,
            });
        }

        Ok(())
    }

    /// 检查路径穿越
    fn is_path_traversal(&self, path: &str) -> bool {
        // 检查 .. 组件
        for component in Path::new(path).components() {
            if let std::path::Component::ParentDir = component {
                return true;
            }
        }

        // 检查绝对路径
        if Path::new(path).is_absolute() {
            return true;
        }

        // 检查可疑模式
        if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
            return true;
        }

        false
    }

    /// 检查文件扩展名是否允许
    fn is_allowed_extension(&self, path: &str) -> bool {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        self.allowed_extensions.iter().any(|allowed| allowed == ext)
    }

    /// 解压到临时目录 (按实际写出字节累计总大小)
    fn extract_to_temp(&self, archive: &mut ZipArchive<File>, temp_dir: &Path) -> Result<()> {
        // 按实际写出字节累计总大小 (不依赖 ZIP 元数据)
        let mut actual_total_size: u64 = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            // 使用 enclosed_name() 安全获取路径 (已在 pre_validate 中验证，这里再次检查以确保安全)
            let safe_name = file.enclosed_name().ok_or_else(|| {
                SecurityError::PathTraversal {
                    path: file.name().to_string(),
                }
            })?;
            let name = safe_name.to_string_lossy().to_string();

            // 构建目标路径 (使用安全路径)
            let target_path = temp_dir.join(&safe_name);

            if file.is_dir() {
                // 创建目录
                fs::create_dir_all(&target_path)?;
            } else {
                // 确保父目录存在
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // 写入文件 (使用 take() 限制读取字节数，防止 ZIP 炸弹)
                let mut outfile = File::create(&target_path)?;
                let mut buffer = Vec::with_capacity(file.size().min(self.max_file_size) as usize);

                // 安全读取：限制最大读取字节数
                // 读取 max_file_size + 1 字节，如果能读到超过 max_file_size，说明文件过大
                let read_limit = self.max_file_size + 1;
                file.take(read_limit).read_to_end(&mut buffer)?;

                // 检查是否超过单文件大小限制（截断检测）
                if buffer.len() as u64 > self.max_file_size {
                    return Err(SecurityError::FileTooLarge {
                        file: name,
                        size: buffer.len() as u64,
                        limit: self.max_file_size,
                    });
                }

                // 累计实际写出字节数
                let bytes_written = buffer.len() as u64;
                actual_total_size += bytes_written;

                // 检查实际总大小是否超限
                if actual_total_size > self.max_total_size {
                    return Err(SecurityError::TotalSizeTooLarge {
                        total: actual_total_size,
                        limit: self.max_total_size,
                    });
                }

                outfile.write_all(&buffer)?;

                log::debug!("解压文件: {} ({} bytes, 累计 {} bytes)", name, bytes_written, actual_total_size);
            }
        }

        Ok(())
    }

    /// 创建备份
    fn create_backup(&self, dir: &Path) -> Result<()> {
        let parent = dir.parent().ok_or_else(|| {
            SecurityError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "无法获取父目录",
            ))
        })?;

        let dir_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                SecurityError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "无法获取目录名",
                ))
            })?;

        // 轮转备份
        self.rotate_backups(parent, dir_name)?;

        // 创建新备份
        let backup_path = parent.join(format!("{}.backup.1", dir_name));
        if backup_path.exists() {
            fs::remove_dir_all(&backup_path)?;
        }

        // 复制目录
        self.copy_dir_recursive(dir, &backup_path)?;

        log::info!("创建备份: {:?}", backup_path);
        Ok(())
    }

    /// 轮转备份 (保留 N 个版本)
    fn rotate_backups(&self, parent: &Path, dir_name: &str) -> Result<()> {
        // 删除最旧的备份
        let oldest = parent.join(format!("{}.backup.{}", dir_name, self.backup_versions));
        if oldest.exists() {
            fs::remove_dir_all(&oldest)?;
        }

        // 依次重命名
        for i in (1..self.backup_versions).rev() {
            let old_path = parent.join(format!("{}.backup.{}", dir_name, i));
            let new_path = parent.join(format!("{}.backup.{}", dir_name, i + 1));
            if old_path.exists() {
                fs::rename(&old_path, &new_path)?;
            }
        }

        Ok(())
    }

    /// 递归复制目录 (安全：拒绝符号链接)
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            // 安全检查：使用 symlink_metadata 检测符号链接 (不跟随符号链接)
            let metadata = src_path.symlink_metadata()?;
            if metadata.is_symlink() {
                return Err(SecurityError::SymlinkRejected {
                    path: src_path.to_string_lossy().to_string(),
                });
            }

            if metadata.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// 原子替换目录
    fn atomic_replace(&self, temp_dir: &Path, target_dir: &Path) -> Result<()> {
        // 如果目标目录存在，先删除
        if target_dir.exists() {
            fs::remove_dir_all(target_dir)?;
        }

        // 确保父目录存在
        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        // 尝试重命名 (同一文件系统内是原子操作)
        match fs::rename(temp_dir, target_dir) {
            Ok(_) => Ok(()),
            Err(e) => {
                // 如果重命名失败 (跨文件系统)，使用复制
                log::warn!("重命名失败，使用复制: {}", e);
                self.copy_dir_recursive(temp_dir, target_dir)?;
                Ok(())
            }
        }
    }

    /// 回滚到最近的备份
    pub fn rollback<P: AsRef<Path>>(&self, target_dir: P) -> Result<()> {
        let target_dir = target_dir.as_ref();
        let parent = target_dir.parent().ok_or_else(|| {
            SecurityError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "无法获取父目录",
            ))
        })?;

        let dir_name = target_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                SecurityError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "无法获取目录名",
                ))
            })?;

        let backup_path = parent.join(format!("{}.backup.1", dir_name));

        if !backup_path.exists() {
            return Err(SecurityError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "没有可用的备份",
            )));
        }

        // 删除当前目录
        if target_dir.exists() {
            fs::remove_dir_all(target_dir)?;
        }

        // 恢复备份
        fs::rename(&backup_path, target_dir)?;

        log::info!("回滚完成: {:?}", target_dir);
        Ok(())
    }

    /// 列出可用备份
    pub fn list_backups<P: AsRef<Path>>(&self, target_dir: P) -> Result<Vec<PathBuf>> {
        let target_dir = target_dir.as_ref();
        let parent = target_dir.parent().ok_or_else(|| {
            SecurityError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "无法获取父目录",
            ))
        })?;

        let dir_name = target_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                SecurityError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "无法获取目录名",
                ))
            })?;

        let mut backups = Vec::new();
        for i in 1..=self.backup_versions {
            let backup_path = parent.join(format!("{}.backup.{}", dir_name, i));
            if backup_path.exists() {
                backups.push(backup_path);
            }
        }

        Ok(backups)
    }
}

impl Default for SecureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();

            for (name, content) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(content).unwrap();
            }

            writer.finish().unwrap();
        }
        buffer
    }

    fn write_test_zip(temp_dir: &TempDir, files: &[(&str, &[u8])]) -> PathBuf {
        let zip_data = create_test_zip(files);
        let zip_path = temp_dir.path().join("test.zip");
        let mut file = File::create(&zip_path).unwrap();
        file.write_all(&zip_data).unwrap();
        zip_path
    }

    #[test]
    fn test_extract_valid_zip() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = write_test_zip(&temp_dir, &[("plugin.js", b"console.log('test');")]);

        let target_dir = temp_dir.path().join("plugin");
        let extractor = SecureExtractor::new();

        assert!(extractor.extract(&zip_path, &target_dir).is_ok());
        assert!(target_dir.join("plugin.js").exists());
    }

    #[test]
    fn test_reject_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = write_test_zip(&temp_dir, &[("../evil.js", b"malicious code")]);

        let target_dir = temp_dir.path().join("plugin");
        let extractor = SecureExtractor::new();

        let result = extractor.extract(&zip_path, &target_dir);
        assert!(matches!(result, Err(SecurityError::PathTraversal { .. })));
    }

    #[test]
    fn test_reject_file_too_large() {
        let temp_dir = TempDir::new().unwrap();

        // 创建一个大文件
        let large_content = vec![0u8; 1024]; // 1KB
        let zip_path = write_test_zip(&temp_dir, &[("large.js", &large_content)]);

        let target_dir = temp_dir.path().join("plugin");
        let extractor = SecureExtractor::new().with_limits(512, 50 * 1024 * 1024); // 512 bytes 限制

        let result = extractor.extract(&zip_path, &target_dir);
        assert!(matches!(result, Err(SecurityError::FileTooLarge { .. })));
    }

    #[test]
    fn test_reject_disallowed_extension() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = write_test_zip(&temp_dir, &[("script.exe", b"malware")]);

        let target_dir = temp_dir.path().join("plugin");
        let extractor = SecureExtractor::new();

        let result = extractor.extract(&zip_path, &target_dir);
        assert!(matches!(result, Err(SecurityError::FileTypeNotAllowed { .. })));
    }

    #[test]
    fn test_backup_and_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("plugin");

        // 创建初始版本
        fs::create_dir_all(&target_dir).unwrap();
        let v1_file = target_dir.join("version.txt");
        fs::write(&v1_file, "v1").unwrap();

        // 解压新版本
        let zip_path = write_test_zip(&temp_dir, &[("plugin.js", b"v2")]);
        let extractor = SecureExtractor::new();
        extractor.extract(&zip_path, &target_dir).unwrap();

        // 验证新版本
        assert!(target_dir.join("plugin.js").exists());

        // 回滚
        extractor.rollback(&target_dir).unwrap();

        // 验证回滚
        assert!(v1_file.exists());
        assert_eq!(fs::read_to_string(&v1_file).unwrap(), "v1");
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("plugin");
        let extractor = SecureExtractor::new().with_backup_versions(3);

        // 创建初始版本
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("v1.txt"), "v1").unwrap();

        // 多次更新
        for i in 2..=4 {
            let zip_path =
                write_test_zip(&temp_dir, &[("plugin.js", format!("v{}", i).as_bytes())]);
            extractor.extract(&zip_path, &target_dir).unwrap();
        }

        // 检查备份数量
        let backups = extractor.list_backups(&target_dir).unwrap();
        assert!(backups.len() <= 3);
    }

    #[test]
    fn test_is_path_traversal() {
        let extractor = SecureExtractor::new();

        assert!(extractor.is_path_traversal("../evil.js"));
        assert!(extractor.is_path_traversal("foo/../bar.js"));
        assert!(extractor.is_path_traversal("/etc/passwd"));
        assert!(extractor.is_path_traversal("\\Windows\\System32"));

        assert!(!extractor.is_path_traversal("plugin.js"));
        assert!(!extractor.is_path_traversal("src/index.js"));
        assert!(!extractor.is_path_traversal("assets/icon.png"));
    }

    #[test]
    fn test_is_allowed_extension() {
        let extractor = SecureExtractor::new();

        assert!(extractor.is_allowed_extension("plugin.js"));
        assert!(extractor.is_allowed_extension("manifest.json"));
        assert!(extractor.is_allowed_extension("icon.png"));
        assert!(extractor.is_allowed_extension("logo.svg"));

        assert!(!extractor.is_allowed_extension("script.exe"));
        assert!(!extractor.is_allowed_extension("payload.sh"));
        assert!(!extractor.is_allowed_extension("config.yaml"));
    }
}
