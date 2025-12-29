# API Version 兼容策略

> 版本: 1.0.0
> 冻结时间: 2025-12-27
> 状态: FROZEN

## 1. 版本格式

```
apiVersion: "major.minor"
```

- **major**: 主版本号，不兼容的 API 变更
- **minor**: 次版本号，向后兼容的功能新增

## 2. 兼容性规则

### 2.1 运行时支持的 API 版本范围

```
支持范围: [CURRENT_MAJOR.0, CURRENT_MAJOR.CURRENT_MINOR]
```

运行时维护一个当前支持的 API 版本：

| 运行时版本 | 支持的 apiVersion |
|-----------|------------------|
| 1.0.0 | 1.0 |
| 1.1.0 | 1.0, 1.1 |
| 1.2.0 | 1.0, 1.1, 1.2 |
| 2.0.0 | 2.0 (不兼容 1.x) |

### 2.2 Major 版本升级规则

Major 版本升级 (如 1.x → 2.x) 触发条件：

1. **PluginContext API 破坏性变更**
   - 移除现有方法
   - 更改方法签名（参数/返回值类型）
   - 更改 storage/cache 键空间行为

2. **IPC 契约破坏性变更**
   - 移除现有 Command
   - 更改 Command 参数/返回类型
   - 更改 Event Payload 结构

3. **manifest.json 破坏性变更**
   - 移除必填字段
   - 更改字段类型
   - 更改签名算法

**处理策略**：
- Major 升级时，运行时**不再支持**旧 Major 版本插件
- 必须发布迁移指南和升级工具
- 给予至少 30 天过渡期公告

### 2.3 Minor 版本升级规则

Minor 版本升级 (如 1.0 → 1.1) 触发条件：

1. **新增 PluginContext 方法**（不影响现有方法）
2. **新增 IPC Command/Event**（不影响现有接口）
3. **新增 manifest.json 可选字段**
4. **新增数据类型**（如新的 dataType）

**处理策略**：
- Minor 升级保持**完全向后兼容**
- 旧 Minor 版本插件在新运行时正常工作
- 新功能仅在声明新 Minor 版本时可用

## 3. 插件加载时的版本检查

```rust
fn check_api_version(plugin_api_version: &str, runtime_api_version: &str) -> Result<(), PluginError> {
    let (plugin_major, plugin_minor) = parse_version(plugin_api_version)?;
    let (runtime_major, runtime_minor) = parse_version(runtime_api_version)?;

    // Major 版本必须完全匹配
    if plugin_major != runtime_major {
        return Err(PluginError::new(
            PluginErrorType::IncompatibleApiVersion,
            format!(
                "插件 API 版本 {} 与运行时版本 {} 不兼容，需要升级插件",
                plugin_api_version, runtime_api_version
            )
        ));
    }

    // Minor 版本：插件可以低于运行时
    if plugin_minor > runtime_minor {
        return Err(PluginError::new(
            PluginErrorType::IncompatibleApiVersion,
            format!(
                "插件需要 API 版本 {}，但运行时仅支持 {}，请升级应用",
                plugin_api_version, runtime_api_version
            )
        ));
    }

    Ok(())
}
```

## 4. 版本降级行为

### 4.1 插件使用旧 Minor 版本

- **行为**: 正常加载，旧 API 完全可用
- **新功能门控**: 运行时**不暴露**新 Minor 版本引入的方法/事件
- **能力探测**: 插件可通过 `context.hasCapability(name)` 检查功能是否可用
- **调用不存在的 API**: 抛出 `PluginError(UNSUPPORTED_API, "API xxx requires apiVersion >= 1.x")`

```javascript
// 插件中的安全调用模式
if (context.hasCapability('cache.has')) {
  const exists = await context.cache.has('key');
} else {
  // 降级处理：使用 get 并检查 null
  const value = await context.cache.get('key');
  const exists = value !== null;
}
```

### 4.2 插件使用新 Minor 版本

- **行为**: 拒绝加载
- **错误类型**: `PluginError(INCOMPATIBLE_API_VERSION, ...)`
- **错误信息**: `"插件需要 API 版本 1.2，但运行时仅支持 1.1，请升级应用"`
- **用户操作**: 提示升级应用或降级插件

### 4.3 插件使用不同 Major 版本

- **行为**: 拒绝加载
- **错误类型**: `PluginError(INCOMPATIBLE_API_VERSION, ...)`
- **错误信息**: `"插件 API 版本 2.0 与运行时版本 1.x 不兼容"`
- **用户操作**: 提供迁移指南链接

### 4.4 能力探测 API

运行时提供能力探测方法，避免插件调用不存在的 API：

```typescript
interface PluginContext {
  /**
   * 检查指定能力是否可用
   * @param capability 能力名称，格式: "{namespace}.{method}"
   * @example context.hasCapability('cache.has') // 检查 cache.has 是否可用
   */
  hasCapability(capability: string): boolean;

  /**
   * 获取当前运行时支持的 API 版本
   */
  readonly runtimeApiVersion: string;
}
```

### 4.5 能力引入版本映射

| 能力 | 引入版本 | 说明 |
|------|----------|------|
| `storage.keys` | 1.0 | 列出所有键 |
| `storage.clear` | 1.0 | 清空存储 |
| `cache.has` | 1.0 | 检查缓存是否存在 |
| `context.hasCapability` | 1.0 | 能力探测 |

## 5. 变更流程

任何契约变更必须遵循以下流程：

1. **提案**: 创建 RFC 描述变更内容和影响
2. **审核**: 至少 2 人审核通过
3. **版本号**: 确定是 Major 还是 Minor 升级
4. **文档**: 更新所有相关契约文档
5. **类型**: 更新 TypeScript 类型定义
6. **Tag**: Git Tag 标记新版本
7. **公告**: 发布变更公告（Major 变更需要 30 天公告期）

## 6. 当前版本

```
API Version: 1.0
Runtime Version: 1.0.0
Frozen Date: 2025-12-27
```
