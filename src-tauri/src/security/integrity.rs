// Phase 5A.2: SHA256 完整性校验
// 验证插件文件的完整性

use crate::security::{Result, SecurityError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path};

// ============================================================================
// SHA256 计算
// ============================================================================

/// 计算文件的 SHA256 哈希值
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// - `Ok(String)`: 十六进制格式的 SHA256 哈希值
/// - `Err(SecurityError)`: 读取文件失败
pub fn calculate_sha256<P: AsRef<Path>>(path: P) -> Result<String> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// 计算字节数据的 SHA256 哈希值
///
/// # 参数
/// - `data`: 字节数据
///
/// # 返回
/// - 十六进制格式的 SHA256 哈希值
pub fn calculate_sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

// ============================================================================
// 路径安全校验
// ============================================================================

/// 检查文件路径是否安全（不包含路径穿越）
///
/// 拒绝以下路径：
/// - 包含 `..` (ParentDir)
/// - 绝对路径 (RootDir/Prefix)
/// - 空路径
fn is_safe_filename(filename: &str) -> bool {
    if filename.is_empty() {
        return false;
    }

    let path = Path::new(filename);

    // 检查是否为绝对路径
    if path.is_absolute() {
        return false;
    }

    // 检查路径组件
    for component in path.components() {
        match component {
            // 拒绝父目录引用
            Component::ParentDir => return false,
            // 拒绝根目录
            Component::RootDir => return false,
            // 拒绝 Windows 前缀 (如 C:)
            Component::Prefix(_) => return false,
            // 允许正常组件和当前目录
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    true
}

// ============================================================================
// 文件哈希验证
// ============================================================================

/// 验证单个文件的哈希值
///
/// # 参数
/// - `path`: 文件路径
/// - `expected_hash`: 期望的哈希值 (格式: "sha256:{hex64}" 或纯 hex64)
///
/// # 返回
/// - `Ok(())`: 哈希匹配
/// - `Err(SecurityError)`: 哈希不匹配或文件读取失败
pub fn verify_file_hash<P: AsRef<Path>>(path: P, expected_hash: &str) -> Result<()> {
    let path = path.as_ref();

    // 解析期望哈希 (支持 "sha256:{hex}" 或纯 hex 格式)
    let expected = expected_hash.strip_prefix("sha256:").unwrap_or(expected_hash);

    // 计算实际哈希
    let actual = calculate_sha256(path)?;

    if actual != expected {
        return Err(SecurityError::HashMismatch {
            file: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }

    Ok(())
}

/// 验证 manifest.files 中的所有文件哈希
///
/// # 参数
/// - `manifest`: manifest.json 内容
/// - `plugin_dir`: 插件目录路径
///
/// # 返回
/// - `Ok(())`: 所有文件哈希匹配
/// - `Err(SecurityError)`: 哈希不匹配或文件缺失
pub fn verify_manifest_files<P: AsRef<Path>>(manifest: &Value, plugin_dir: P) -> Result<()> {
    let plugin_dir = plugin_dir.as_ref();

    // 获取 files 字段
    let files = match manifest.get("files") {
        Some(Value::Object(map)) => map,
        Some(_) => {
            return Err(SecurityError::SignatureFormatError {
                expected: "object",
                got: "files field is not an object".to_string(),
            })
        }
        None => {
            // 没有 files 字段，跳过验证
            log::debug!("manifest 没有 files 字段，跳过文件哈希验证");
            return Ok(());
        }
    };

    // 验证每个文件
    for (filename, hash_value) in files {
        // 安全检查：拒绝路径穿越
        if !is_safe_filename(filename) {
            return Err(SecurityError::PathTraversal {
                path: filename.clone(),
            });
        }

        let hash = hash_value.as_str().ok_or_else(|| SecurityError::SignatureFormatError {
            expected: "string",
            got: format!("hash for {} is not a string", filename),
        })?;

        let file_path = plugin_dir.join(filename);

        // 检查文件是否存在
        if !file_path.exists() {
            return Err(SecurityError::FileMissing {
                file: filename.clone(),
            });
        }

        // 验证哈希
        verify_file_hash(&file_path, hash)?;

        log::debug!("文件验证通过: {}", filename);
    }

    Ok(())
}

/// 生成目录中所有文件的哈希映射
///
/// # 参数
/// - `dir`: 目录路径
/// - `extensions`: 要包含的文件扩展名 (None 表示包含所有)
///
/// # 返回
/// - 文件名 -> "sha256:{hex}" 的映射
pub fn generate_file_hashes<P: AsRef<Path>>(
    dir: P,
    extensions: Option<&[&str]>,
) -> Result<HashMap<String, String>> {
    let dir = dir.as_ref();
    let mut hashes = HashMap::new();

    if !dir.is_dir() {
        return Err(SecurityError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("目录不存在: {:?}", dir),
        )));
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // 跳过目录
        if path.is_dir() {
            continue;
        }

        // 检查扩展名
        if let Some(exts) = extensions {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !exts.contains(&ext) {
                continue;
            }
        }

        // 计算哈希
        let filename = entry.file_name().to_string_lossy().to_string();
        let hash = calculate_sha256(&path)?;
        hashes.insert(filename, format!("sha256:{}", hash));
    }

    Ok(hashes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_sha256() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();

        let hash = calculate_sha256(&file_path).unwrap();
        // SHA256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_calculate_sha256_bytes() {
        let hash = calculate_sha256_bytes(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verify_file_hash_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();

        let expected = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_file_hash(&file_path, expected).is_ok());
    }

    #[test]
    fn test_verify_file_hash_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();

        let wrong_hash = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_file_hash(&file_path, wrong_hash);
        assert!(matches!(result, Err(SecurityError::HashMismatch { .. })));
    }

    #[test]
    fn test_verify_manifest_files() {
        let temp_dir = TempDir::new().unwrap();

        // 创建测试文件
        let plugin_path = temp_dir.path().join("plugin.js");
        let mut file = File::create(&plugin_path).unwrap();
        file.write_all(b"console.log('hello');").unwrap();

        let hash = calculate_sha256(&plugin_path).unwrap();

        let manifest = json!({
            "id": "test-plugin",
            "files": {
                "plugin.js": format!("sha256:{}", hash)
            }
        });

        assert!(verify_manifest_files(&manifest, temp_dir.path()).is_ok());
    }

    #[test]
    fn test_verify_manifest_files_missing() {
        let temp_dir = TempDir::new().unwrap();

        let manifest = json!({
            "id": "test-plugin",
            "files": {
                "missing.js": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }
        });

        let result = verify_manifest_files(&manifest, temp_dir.path());
        assert!(matches!(result, Err(SecurityError::FileMissing { .. })));
    }

    #[test]
    fn test_verify_manifest_no_files_field() {
        let temp_dir = TempDir::new().unwrap();

        let manifest = json!({
            "id": "test-plugin"
        });

        // 没有 files 字段时应该通过
        assert!(verify_manifest_files(&manifest, temp_dir.path()).is_ok());
    }

    #[test]
    fn test_generate_file_hashes() {
        let temp_dir = TempDir::new().unwrap();

        // 创建测试文件
        let js_path = temp_dir.path().join("plugin.js");
        let mut js_file = File::create(&js_path).unwrap();
        js_file.write_all(b"console.log('test');").unwrap();

        let json_path = temp_dir.path().join("manifest.json");
        let mut json_file = File::create(&json_path).unwrap();
        json_file.write_all(b"{}").unwrap();

        let txt_path = temp_dir.path().join("readme.txt");
        let mut txt_file = File::create(&txt_path).unwrap();
        txt_file.write_all(b"readme").unwrap();

        // 只包含 .js 文件
        let hashes = generate_file_hashes(temp_dir.path(), Some(&["js"])).unwrap();
        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key("plugin.js"));

        // 包含所有文件
        let all_hashes = generate_file_hashes(temp_dir.path(), None).unwrap();
        assert_eq!(all_hashes.len(), 3);
    }
}
