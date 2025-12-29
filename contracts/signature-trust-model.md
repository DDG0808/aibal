# 签名信任模型

> 版本: 1.0.0
> 创建时间: 2025-12-27
> 状态: FROZEN

## 1. 概述

本文档定义 CUK 插件系统的签名信任模型，明确公钥来源、签名覆盖范围和验证流程。

## 2. 信任模型

### 2.1 信任根

CUK 采用**单一信任根**模型：

```
┌─────────────────────────────────────────────────────────┐
│  CUK 应用二进制                                          │
│  ├── 内嵌官方公钥 (Ed25519)                              │
│  └── 公钥 ID: "cuk-official-2025"                       │
└─────────────────────────────────────────────────────────┘
                        │
                        │ 验证
                        ▼
┌─────────────────────────────────────────────────────────┐
│  官方插件 manifest.json                                  │
│  ├── signature: "ed25519:..."                           │
│  └── 由官方私钥签名                                      │
└─────────────────────────────────────────────────────────┘
```

### 2.2 公钥来源

| 来源 | 说明 | 信任级别 |
|------|------|----------|
| **内嵌公钥** | 编译时嵌入应用二进制 | 最高 (官方插件) |
| **用户导入** | 用户手动信任的第三方公钥 | 用户授权 |
| **无签名** | 本地开发插件 | 需用户确认 |

### 2.3 公钥存储

```rust
// 编译时嵌入 (src/security/keys.rs)
pub const OFFICIAL_PUBLIC_KEY: &[u8; 32] = include_bytes!("../../keys/official.pub");
pub const OFFICIAL_KEY_ID: &str = "cuk-official-2025";

// 用户导入的公钥存储位置
// ~/.config/cuk/trusted-keys/
// ├── {key_id}.pub          # Ed25519 公钥 (32 bytes)
// └── trusted-keys.json     # 元数据
```

## 3. 签名覆盖范围

### 3.1 签名对象

签名覆盖 `manifest.json` 的**规范化内容**（不含 `signature` 字段）：

```json
{
  "id": "claude-usage",
  "name": "Claude 使用量",
  "version": "1.0.0",
  "apiVersion": "1.0",
  "pluginType": "data",
  "files": {
    "plugin.js": "sha256:abc123...",
    "icon.png": "sha256:def456..."
  }
  // signature 字段在签名时被排除
}
```

### 3.2 文件完整性

通过 `files` 字段间接覆盖所有插件文件：

```
manifest.json (签名覆盖)
    │
    ├── files.plugin.js: "sha256:abc123..."
    │       │
    │       └── 验证 plugin.js 的 SHA-256
    │
    └── files.icon.png: "sha256:def456..."
            │
            └── 验证 icon.png 的 SHA-256
```

### 3.3 验证顺序

```
1. 读取 manifest.json
2. 提取 signature 字段
3. 移除 signature 字段，规范化剩余内容
4. 使用公钥验证签名
5. 签名验证通过后，逐个验证 files 中的哈希
6. 全部通过则允许加载
```

## 4. signature 字段规范

### 4.1 格式

```
signature: "{algorithm}:{key_id}:{base64_signature}"
```

| 组成部分 | 说明 | 示例 |
|----------|------|------|
| algorithm | 签名算法 | `ed25519` |
| key_id | 公钥标识符 | `cuk-official-2025` |
| base64_signature | Base64 编码的签名 | `ABC123...` |

### 4.2 完整示例

```json
{
  "id": "claude-usage",
  "version": "1.0.0",
  "signature": "ed25519:cuk-official-2025:MEUCIQDf...base64..."
}
```

### 4.3 Base64 规范

- 使用**标准 Base64** (RFC 4648)
- **包含 padding** (`=`)
- 字符集: `A-Za-z0-9+/=`

## 5. 验证流程

### 5.1 官方插件验证

```rust
fn verify_official_plugin(manifest: &Manifest) -> Result<(), PluginError> {
    // 1. 解析 signature 字段
    let sig = parse_signature(&manifest.signature)?;

    // 2. 检查 key_id 是否为官方
    if sig.key_id != OFFICIAL_KEY_ID {
        return Err(PluginError::new(
            PluginErrorType::SIGNATURE_INVALID,
            "Unknown key_id for official plugin"
        ));
    }

    // 3. 移除 signature 字段并规范化
    let canonical = canonicalize_without_signature(manifest)?;

    // 4. 使用内嵌公钥验证
    verify_ed25519(OFFICIAL_PUBLIC_KEY, &canonical, &sig.signature)?;

    // 5. 验证文件哈希
    for (file, expected_hash) in &manifest.files {
        verify_file_hash(file, expected_hash)?;
    }

    Ok(())
}
```

### 5.2 第三方插件验证

```rust
fn verify_third_party_plugin(manifest: &Manifest) -> Result<(), PluginError> {
    let sig = parse_signature(&manifest.signature)?;

    // 查找用户信任的公钥
    let public_key = load_trusted_key(&sig.key_id)?;

    if public_key.is_none() {
        // 未知签名者，需要用户确认
        return Err(PluginError::new(
            PluginErrorType::SIGNATURE_UNTRUSTED,
            format!("Unknown signer: {}", sig.key_id)
        ));
    }

    // 验证签名和文件哈希
    verify_signature_and_files(manifest, public_key.unwrap())
}
```

### 5.3 无签名插件

```rust
fn handle_unsigned_plugin(manifest: &Manifest) -> Result<(), PluginError> {
    if manifest.signature.is_none() {
        // 无签名插件需要用户明确确认
        return Err(PluginError::new(
            PluginErrorType::SIGNATURE_MISSING,
            "Plugin is not signed. User confirmation required."
        ));
    }
    Ok(())
}
```

## 6. 用户信任管理

### 6.1 trusted-keys.json 格式

```json
{
  "version": 1,
  "keys": [
    {
      "id": "community-dev-alice",
      "name": "Alice's Development Key",
      "algorithm": "ed25519",
      "publicKey": "base64...",
      "addedAt": "2025-12-27T10:00:00Z",
      "addedBy": "user"
    }
  ]
}
```

### 6.2 信任操作

| 操作 | 说明 | 风险 |
|------|------|------|
| 添加公钥 | 用户导入第三方公钥 | 中 (需审核来源) |
| 移除公钥 | 撤销信任 | 低 |
| 临时信任 | 仅本次安装信任 | 高 |

## 7. 安全考虑

### 7.1 密钥轮换

- 官方公钥轮换需要发布新版应用
- 旧公钥在轮换后保留 180 天兼容期
- 轮换时 `key_id` 必须更新 (如 `cuk-official-2026`)

### 7.2 私钥保护

- 官方私钥离线存储，仅在发布时使用
- 使用 HSM 或安全密钥存储
- 私钥泄露时立即轮换并公告

### 7.3 降级攻击防护

- 不允许降级到无签名版本
- 不允许降级 key_id 版本

## 8. 变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2025-12-27 | 初始版本，基于 Codex 审核建议创建 |
