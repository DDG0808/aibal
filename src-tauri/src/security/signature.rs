// Phase 5A.1: Ed25519 签名验证
// 验证 manifest.json 的数字签名

use crate::security::canonical::canonicalize_for_signing;
use crate::security::{Result, SecurityError};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// 嵌入公钥
// ============================================================================

/// 官方公钥 (编译时内联)
/// 格式: key_id -> 32字节公钥
///
/// 这是正式签名密钥，用于验证官方发布的插件。
/// 私钥由项目维护者安全保管，用于签名插件 manifest。
///
/// 如需更换密钥，可通过以下方式生成新密钥对:
/// ```bash
/// cargo run --example gen_keys
/// ```

// 官方签名密钥 (正式密钥)
// 生成日期: 2025-12-28
// 私钥路径: .claude/keys/official-key-20251228.md (安全保管，勿提交 Git!)
const OFFICIAL_PUBLIC_KEY: (&str, [u8; PUBLIC_KEY_LENGTH]) = (
    "official",
    [
        0x36, 0x24, 0xaa, 0xa1, 0x5c, 0x61, 0x3d, 0xd7, 0x76, 0x46, 0x44, 0x4b, 0xe8, 0xe4,
        0x96, 0x99, 0x4d, 0x1e, 0x78, 0x2a, 0x10, 0x60, 0xc1, 0xc2, 0x47, 0xc3, 0x66, 0x54,
        0xd7, 0x11, 0x8a, 0xa8,
    ],
);

// 测试密钥 (仅在测试模式下可用，防止被用于签名伪造)
// 使用 RFC 8032 测试向量中的密钥对
// 私钥 (测试模块): 9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60
// 公钥: d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a
#[cfg(test)]
const TEST_PUBLIC_KEY: (&str, [u8; PUBLIC_KEY_LENGTH]) = (
    "test",
    [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
        0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
        0xf7, 0x07, 0x51, 0x1a,
    ],
);

/// 生产构建：仅包含官方密钥
#[cfg(not(test))]
const EMBEDDED_PUBLIC_KEYS: &[(&str, [u8; PUBLIC_KEY_LENGTH])] = &[OFFICIAL_PUBLIC_KEY];

/// 测试构建：包含官方密钥和测试密钥
#[cfg(test)]
const EMBEDDED_PUBLIC_KEYS: &[(&str, [u8; PUBLIC_KEY_LENGTH])] =
    &[OFFICIAL_PUBLIC_KEY, TEST_PUBLIC_KEY];

// ============================================================================
// 签名验证器
// ============================================================================

/// Ed25519 签名验证器
pub struct SignatureVerifier {
    /// 公钥映射表
    keys: HashMap<String, VerifyingKey>,
}

impl SignatureVerifier {
    /// 创建验证器实例，加载嵌入的公钥
    pub fn new() -> Result<Self> {
        let mut keys = HashMap::new();

        for (key_id, key_bytes) in EMBEDDED_PUBLIC_KEYS {
            match VerifyingKey::from_bytes(key_bytes) {
                Ok(key) => {
                    keys.insert(key_id.to_string(), key);
                }
                Err(e) => {
                    log::warn!("加载公钥 {} 失败: {}", key_id, e);
                }
            }
        }

        Ok(Self { keys })
    }

    /// 验证 manifest 签名
    ///
    /// # 参数
    /// - `manifest`: manifest.json 内容 (包含 signature 字段)
    ///
    /// # 返回
    /// - `Ok(())`: 签名验证通过
    /// - `Err(SecurityError)`: 签名验证失败
    pub fn verify_manifest(&self, manifest: &Value) -> Result<()> {
        // 1. 提取 signature 字段
        let signature_str = manifest
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SecurityError::SignatureFormatError {
                expected: "ed25519:{key_id}:{base64}",
                got: "missing signature field".to_string(),
            })?;

        // 2. 解析签名格式: ed25519:{key_id}:{base64}
        let (key_id, signature) = self.parse_signature(signature_str)?;

        // 3. 获取对应公钥
        let verifying_key = self.keys.get(&key_id).ok_or_else(|| {
            SecurityError::PublicKeyNotFound {
                key_id: key_id.clone(),
            }
        })?;

        // 4. 规范化 manifest (移除 signature 字段)
        let canonical = canonicalize_for_signing(manifest);

        // 5. 验证签名
        verifying_key
            .verify_strict(canonical.as_bytes(), &signature)
            .map_err(|e| SecurityError::SignatureInvalid {
                reason: e.to_string(),
            })
    }

    /// 解析签名字符串
    ///
    /// 格式: ed25519:{key_id}:{base64_signature}
    fn parse_signature(&self, signature: &str) -> Result<(String, Signature)> {
        let parts: Vec<&str> = signature.split(':').collect();
        if parts.len() != 3 {
            return Err(SecurityError::SignatureFormatError {
                expected: "ed25519:{key_id}:{base64}",
                got: signature.to_string(),
            });
        }

        if parts[0] != "ed25519" {
            return Err(SecurityError::SignatureFormatError {
                expected: "ed25519:{key_id}:{base64}",
                got: format!("algorithm '{}' not supported", parts[0]),
            });
        }

        let key_id = parts[1].to_string();

        // 解码 Base64 签名
        let signature_bytes = BASE64.decode(parts[2]).map_err(|e| {
            SecurityError::Base64Error(format!("签名 Base64 解码失败: {}", e))
        })?;

        if signature_bytes.len() != SIGNATURE_LENGTH {
            return Err(SecurityError::SignatureFormatError {
                expected: "64 bytes signature",
                got: format!("{} bytes", signature_bytes.len()),
            });
        }

        let mut sig_array = [0u8; SIGNATURE_LENGTH];
        sig_array.copy_from_slice(&signature_bytes);

        let signature = Signature::from_bytes(&sig_array);

        Ok((key_id, signature))
    }

    /// 检查是否有指定的公钥
    pub fn has_key(&self, key_id: &str) -> bool {
        self.keys.contains_key(key_id)
    }

    /// 获取所有可用的公钥 ID
    pub fn available_keys(&self) -> Vec<&String> {
        self.keys.keys().collect()
    }
}

impl Default for SignatureVerifier {
    fn default() -> Self {
        Self::new().expect("Failed to create SignatureVerifier")
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 验证 manifest 签名 (便捷函数)
///
/// # 参数
/// - `manifest`: manifest.json 内容
///
/// # 返回
/// - `Ok(())`: 签名验证通过
/// - `Err(SecurityError)`: 签名验证失败
pub fn verify_manifest_signature(manifest: &Value) -> Result<()> {
    let verifier = SignatureVerifier::new()?;
    verifier.verify_manifest(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    // 测试用私钥 (对应 EMBEDDED_PUBLIC_KEYS 中的 "test" 公钥)
    const TEST_PRIVATE_KEY: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];

    fn create_test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&TEST_PRIVATE_KEY)
    }

    fn sign_manifest(manifest: &Value, key_id: &str) -> String {
        let signing_key = create_test_signing_key();
        let canonical = canonicalize_for_signing(manifest);
        let signature = signing_key.sign(canonical.as_bytes());
        let sig_base64 = BASE64.encode(signature.to_bytes());
        format!("ed25519:{}:{}", key_id, sig_base64)
    }

    #[test]
    fn test_verifier_creation() {
        let verifier = SignatureVerifier::new().unwrap();
        assert!(verifier.has_key("test"));
        assert!(verifier.has_key("official"));
    }

    #[test]
    fn test_valid_signature() {
        let mut manifest = json!({
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0"
        });

        // 添加签名
        let signature = sign_manifest(&manifest, "test");
        manifest["signature"] = serde_json::Value::String(signature);

        let verifier = SignatureVerifier::new().unwrap();
        assert!(verifier.verify_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let manifest = json!({
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "signature": "ed25519:test:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        });

        let verifier = SignatureVerifier::new().unwrap();
        assert!(verifier.verify_manifest(&manifest).is_err());
    }

    #[test]
    fn test_missing_signature() {
        let manifest = json!({
            "id": "test-plugin",
            "name": "Test Plugin"
        });

        let verifier = SignatureVerifier::new().unwrap();
        let result = verifier.verify_manifest(&manifest);
        assert!(matches!(result, Err(SecurityError::SignatureFormatError { .. })));
    }

    #[test]
    fn test_unknown_key_id() {
        // 使用有效的 64 字节签名格式，但 key_id 未知
        // Base64 编码 64 字节需要 88 个字符 (含填充)
        // 64 bytes = 512 bits, Base64 每 6 bits = 1 char, 所以需要 86 chars + 2 填充 = 88 chars
        let valid_sig_base64 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==";
        let manifest = json!({
            "id": "test-plugin",
            "signature": format!("ed25519:unknown:{}", valid_sig_base64)
        });

        let verifier = SignatureVerifier::new().unwrap();
        let result = verifier.verify_manifest(&manifest);
        assert!(
            matches!(result, Err(SecurityError::PublicKeyNotFound { .. })),
            "Expected PublicKeyNotFound, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tampered_manifest() {
        let mut manifest = json!({
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0"
        });

        // 签名原始 manifest
        let signature = sign_manifest(&manifest, "test");
        manifest["signature"] = serde_json::Value::String(signature);

        // 篡改 manifest
        manifest["name"] = serde_json::Value::String("Hacked Plugin".to_string());

        let verifier = SignatureVerifier::new().unwrap();
        let result = verifier.verify_manifest(&manifest);
        assert!(matches!(result, Err(SecurityError::SignatureInvalid { .. })));
    }

    #[test]
    fn test_signature_format_error() {
        let manifest = json!({
            "id": "test-plugin",
            "signature": "invalid-format"
        });

        let verifier = SignatureVerifier::new().unwrap();
        let result = verifier.verify_manifest(&manifest);
        assert!(matches!(result, Err(SecurityError::SignatureFormatError { .. })));
    }
}
