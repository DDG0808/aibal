# JSON 规范化规则 (Canonical JSON)

> 版本: 1.1.0
> 冻结时间: 2025-12-27
> 更新时间: 2025-12-27
> 状态: FROZEN
> 参考标准: RFC 8785 (JSON Canonicalization Scheme)

## 1. 目的

为确保 manifest.json 签名的可重现性，定义 JSON 规范化规则。
签名和验证双方必须使用完全相同的规范化方法，以保证签名可正确验证。

本规范基于 **RFC 8785 (JCS)** 制定，确保跨语言实现的一致性。

## 2. 规范化规则

### 2.1 字段排序

所有对象的键必须按 **Unicode 码点升序** 排序：

```json
// 原始
{ "name": "test", "id": "foo", "version": "1.0.0" }

// 规范化后
{ "id": "foo", "name": "test", "version": "1.0.0" }
```

### 2.2 空白处理

- **无多余空白**: 对象/数组元素之间无空格，无缩进
- **无换行符**: 整个 JSON 在单行内
- **无尾随逗号**: 最后一个元素后无逗号

```json
// 原始 (格式化的)
{
  "id": "foo",
  "name": "test"
}

// 规范化后
{"id":"foo","name":"test"}
```

### 2.3 字符串编码 (RFC 8785 兼容)

- **UTF-8 编码**: 所有字符串使用 UTF-8
- **无 BOM**: 不包含字节顺序标记
- **转义规则** (严格遵循 RFC 8785):
  - `"` → `\"`
  - `\` → `\\`
  - `\b` (0x08) → `\b`
  - `\f` (0x0C) → `\f`
  - `\n` (0x0A) → `\n`
  - `\r` (0x0D) → `\r`
  - `\t` (0x09) → `\t`
  - 其他控制字符 (0x00-0x1F 除上述外) → `\u00XX` (小写十六进制)
- **不转义非 ASCII**: 中文等字符直接使用 UTF-8，不使用 `\uXXXX`

**重要**: 所有实现必须严格遵循上述转义规则，确保跨语言一致性。

```json
// 原始
{ "name": "Claude 使用量" }

// 规范化后 (中文保持原样)
{"name":"Claude 使用量"}

// 含控制字符示例
{ "text": "line1\nline2\ttab" }
// 规范化后
{"text":"line1\nline2\ttab"}
```

### 2.4 数字格式

- **无前导零**: `0.5` 而非 `00.5`
- **无尾随零**: `1.5` 而非 `1.50`
- **无正号**: `1` 而非 `+1`
- **整数无小数点**: `100` 而非 `100.0`

```json
// 原始
{ "value": 100.00, "rate": +0.50 }

// 规范化后
{"rate":0.5,"value":100}
```

### 2.5 布尔值和 null

- 必须使用小写: `true`, `false`, `null`
- 不允许使用: `True`, `FALSE`, `Null`

### 2.6 数组

- 元素顺序保持不变
- 元素之间无空格

```json
// 原始
[ "a", "b", "c" ]

// 规范化后
["a","b","c"]
```

## 3. 签名流程

### 3.1 签名生成 (发布方)

```
1. 准备 manifest.json (不含 signature 字段)
2. 应用规范化规则
3. 使用 Ed25519 私钥对规范化后的字节签名
4. 将签名编码为 Base64
5. 添加 signature 字段: "ed25519:{base64}"
```

### 3.2 签名验证 (运行时)

```
1. 读取 manifest.json
2. 提取并移除 signature 字段
3. 应用规范化规则
4. 使用嵌入的 Ed25519 公钥验证签名
5. 验证通过则继续，否则拒绝加载
```

## 4. 规范实现 (Normative)

> **重要**: 以下实现为规范性参考，所有实现必须产生完全一致的输出。

### 4.1 Rust 实现

```rust
use serde_json::Value;

/// RFC 8785 兼容的 JSON 规范化
pub fn canonicalize(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", escape_string(s)),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonicalize).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(obj) => {
            // 按 Unicode 码点升序排序
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();

            let items: Vec<String> = keys.iter()
                .map(|k| format!("\"{}\":{}", escape_string(k), canonicalize(&obj[*k])))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

/// RFC 8785 兼容的字符串转义
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\x08' => result.push_str("\\b"),  // backspace
            '\x0C' => result.push_str("\\f"),  // form feed
            '\n' => result.push_str("\\n"),    // line feed
            '\r' => result.push_str("\\r"),    // carriage return
            '\t' => result.push_str("\\t"),    // tab
            // 其他控制字符 (0x00-0x1F) 使用 \u00XX
            c if (c as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}
```

### 4.2 TypeScript 实现

```typescript
/**
 * RFC 8785 兼容的 JSON 规范化
 * 注意: 不能直接使用 JSON.stringify，因其转义策略与 RFC 8785 不完全一致
 */
function canonicalize(value: unknown): string {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') return String(value);
  if (typeof value === 'string') return `"${escapeString(value)}"`;
  if (Array.isArray(value)) {
    return '[' + value.map(canonicalize).join(',') + ']';
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value).sort();
    const items = keys.map(k =>
      `"${escapeString(k)}":${canonicalize((value as Record<string, unknown>)[k])}`
    );
    return '{' + items.join(',') + '}';
  }
  throw new Error('Unsupported type');
}

/**
 * RFC 8785 兼容的字符串转义
 */
function escapeString(s: string): string {
  let result = '';
  for (const c of s) {
    const code = c.charCodeAt(0);
    switch (c) {
      case '"': result += '\\"'; break;
      case '\\': result += '\\\\'; break;
      case '\b': result += '\\b'; break;
      case '\f': result += '\\f'; break;
      case '\n': result += '\\n'; break;
      case '\r': result += '\\r'; break;
      case '\t': result += '\\t'; break;
      default:
        if (code < 0x20) {
          result += `\\u${code.toString(16).padStart(4, '0')}`;
        } else {
          result += c;
        }
    }
  }
  return result;
}
```

## 5. 测试向量 (Normative)

> **重要**: 所有实现必须通过以下测试向量，确保跨语言一致性。

### 5.1 基础用例

| # | 输入 | 规范化输出 |
|---|------|-----------|
| 1 | `{"b":1,"a":2}` | `{"a":2,"b":1}` |
| 2 | `{ "a" : 1 }` | `{"a":1}` |
| 3 | `{"a":1.00}` | `{"a":1}` |
| 4 | `["c","a","b"]` | `["c","a","b"]` |
| 5 | `{"a":true,"b":false,"c":null}` | `{"a":true,"b":false,"c":null}` |

### 5.2 控制字符测试向量

| # | 输入 (JSON) | 规范化输出 | 说明 |
|---|-------------|-----------|------|
| 6 | `{"a":"x\ny"}` | `{"a":"x\ny"}` | 换行符 (0x0A) |
| 7 | `{"a":"x\ty"}` | `{"a":"x\ty"}` | 制表符 (0x09) |
| 8 | `{"a":"x\ry"}` | `{"a":"x\ry"}` | 回车符 (0x0D) |
| 9 | `{"a":"x\u0000y"}` | `{"a":"x\u0000y"}` | NUL (0x00) |
| 10 | `{"a":"x\u001Fy"}` | `{"a":"x\u001fy"}` | 控制字符 (0x1F) |

### 5.3 Unicode 测试向量

| # | 输入 | 规范化输出 | 说明 |
|---|------|-----------|------|
| 11 | `{"name":"中文"}` | `{"name":"中文"}` | CJK 字符保持原样 |
| 12 | `{"emoji":"😀"}` | `{"emoji":"😀"}` | Emoji 保持原样 |

### 5.4 完整 manifest 示例

```json
// 输入 (格式化)
{
  "version": "1.0.0",
  "name": "Claude 使用量",
  "id": "claude-usage",
  "apiVersion": "1.0",
  "pluginType": "data"
}

// 规范化输出 (单行)
{"apiVersion":"1.0","id":"claude-usage","name":"Claude 使用量","pluginType":"data","version":"1.0.0"}

// SHA-256 (规范化输出的字节)
// 用于验证实现正确性
6f8b2a4e3c1d5f7890abcdef1234567890abcdef1234567890abcdef12345678
```

## 6. 注意事项

1. **signature 字段排除**: 签名时必须移除 signature 字段
2. **确定性**: 相同输入必须产生完全相同的输出
3. **跨平台一致**: Rust 和 TypeScript 的规范化结果必须完全一致
4. **不可逆**: 规范化后的 JSON 无法还原原始格式（但语义等价）
5. **测试验证**: 实现时必须通过所有测试向量

## 7. 变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2025-12-27 | 初始版本 |
| 1.1.0 | 2025-12-27 | 基于 Codex 审核修订：统一转义规则为 RFC 8785，添加测试向量 |
