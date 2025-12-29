// 配置管理模块
// Phase 4.2: 配置管理
//
// 实现任务:
// - 4.2.1 实现 configSchema 解析 - 自动提取配置定义
// - 4.2.2 实现配置验证 (validateConfig) - 验证失败有错误信息
// - 4.2.3 实现配置 UI 自动生成 - 根据 schema 渲染表单 (前端)
// - 4.2.4 实现配置变更通知 - 配置更新后通知插件

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::event_bus::EventBus;

// ============================================================================
// 配置 Schema 类型定义
// ============================================================================

/// 配置字段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFieldType {
    String,
    Number,
    Boolean,
    Select,
}

/// 选择项定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    /// 选项值
    pub value: String,
    /// 显示标签
    pub label: String,
}

/// 配置字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    /// 字段类型
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,

    /// 是否必填
    #[serde(default)]
    pub required: bool,

    /// 是否为敏感数据 (存储在 Keychain)
    #[serde(default)]
    pub secret: bool,

    /// 显示标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// 字段描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 默认值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,

    /// 最小值 (type=number)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,

    /// 最大值 (type=number)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,

    /// 选项列表 (type=select)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,
}

/// 配置 Schema (字段名 -> 字段定义)
pub type ConfigSchema = HashMap<String, ConfigField>;

// ============================================================================
// 配置验证结果
// ============================================================================

/// 字段验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidationError {
    /// 字段名
    pub field: String,
    /// 错误消息
    pub message: String,
    /// 错误类型
    pub error_type: ValidationErrorType,
}

/// 验证错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorType {
    /// 必填字段缺失
    Required,
    /// 类型不匹配
    TypeMismatch,
    /// 值超出范围
    OutOfRange,
    /// 无效的选项值
    InvalidOption,
    /// 未知字段
    UnknownField,
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationResult {
    /// 是否有效
    pub valid: bool,
    /// 全局错误消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 字段级错误列表
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub field_errors: Vec<FieldValidationError>,
}

impl ConfigValidationResult {
    /// 创建成功结果
    pub fn success() -> Self {
        Self {
            valid: true,
            message: None,
            field_errors: vec![],
        }
    }

    /// 创建失败结果
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            message: Some(message.into()),
            field_errors: vec![],
        }
    }

    /// 添加字段错误
    pub fn add_field_error(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
        error_type: ValidationErrorType,
    ) {
        self.valid = false;
        self.field_errors.push(FieldValidationError {
            field: field.into(),
            message: message.into(),
            error_type,
        });
    }
}

// ============================================================================
// 配置管理器
// ============================================================================

/// 配置管理器
///
/// 管理插件配置的 Schema 解析、验证和变更通知
pub struct ConfigManager {
    /// 插件 Schema 映射: plugin_id -> ConfigSchema
    schemas: RwLock<HashMap<String, ConfigSchema>>,
    /// 事件总线引用 (用于发送变更通知)
    event_bus: Option<Arc<EventBus>>,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            schemas: RwLock::new(HashMap::new()),
            event_bus: None,
        }
    }

    /// 创建带事件总线的配置管理器
    pub fn with_event_bus(event_bus: Arc<EventBus>) -> Self {
        Self {
            schemas: RwLock::new(HashMap::new()),
            event_bus: Some(event_bus),
        }
    }

    // ========================================================================
    // Schema 管理 (4.2.1)
    // ========================================================================

    /// 注册插件配置 Schema
    ///
    /// 从插件 manifest 中解析 configSchema 字段
    pub async fn register_schema(&self, plugin_id: &str, schema: ConfigSchema) {
        self.schemas.write().await.insert(plugin_id.to_string(), schema);
        log::debug!("已注册插件 {} 的配置 Schema", plugin_id);
    }

    /// 从 JSON 值解析并注册 Schema
    ///
    /// manifest.json 中的 configSchema 字段
    pub async fn register_schema_from_json(
        &self,
        plugin_id: &str,
        json: &serde_json::Value,
    ) -> Result<(), String> {
        if json.is_null() {
            // 无配置 Schema
            return Ok(());
        }

        let schema: ConfigSchema = serde_json::from_value(json.clone())
            .map_err(|e| format!("解析 configSchema 失败: {}", e))?;

        self.register_schema(plugin_id, schema).await;
        Ok(())
    }

    /// 获取插件配置 Schema
    pub async fn get_schema(&self, plugin_id: &str) -> Option<ConfigSchema> {
        self.schemas.read().await.get(plugin_id).cloned()
    }

    /// 移除插件配置 Schema
    pub async fn unregister_schema(&self, plugin_id: &str) {
        self.schemas.write().await.remove(plugin_id);
        log::debug!("已移除插件 {} 的配置 Schema", plugin_id);
    }

    // ========================================================================
    // 配置验证 (4.2.2)
    // ========================================================================

    /// 验证配置值
    ///
    /// 根据已注册的 Schema 验证配置值的类型和约束
    pub async fn validate(
        &self,
        plugin_id: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> ConfigValidationResult {
        let schemas = self.schemas.read().await;

        let schema = match schemas.get(plugin_id) {
            Some(s) => s,
            None => {
                // 无 Schema，直接通过
                return ConfigValidationResult::success();
            }
        };

        let mut result = ConfigValidationResult::success();

        // 验证每个 Schema 字段
        for (field_name, field_def) in schema {
            let value = config.get(field_name);

            // 检查必填字段
            if field_def.required {
                if value.is_none() || value.map(|v| v.is_null()).unwrap_or(true) {
                    result.add_field_error(
                        field_name,
                        format!("字段 {} 为必填项", field_name),
                        ValidationErrorType::Required,
                    );
                    continue;
                }
            }

            // 如果值不存在且非必填，跳过
            let value = match value {
                Some(v) if !v.is_null() => v,
                _ => continue,
            };

            // 验证类型
            match field_def.field_type {
                ConfigFieldType::String => {
                    if !value.is_string() {
                        result.add_field_error(
                            field_name,
                            format!("字段 {} 应为字符串类型", field_name),
                            ValidationErrorType::TypeMismatch,
                        );
                    }
                }
                ConfigFieldType::Number => {
                    if !value.is_number() {
                        result.add_field_error(
                            field_name,
                            format!("字段 {} 应为数字类型", field_name),
                            ValidationErrorType::TypeMismatch,
                        );
                    } else {
                        // 检查范围
                        let num = value.as_f64().unwrap();

                        if let Some(min) = field_def.min {
                            if num < min {
                                result.add_field_error(
                                    field_name,
                                    format!("字段 {} 的值不能小于 {}", field_name, min),
                                    ValidationErrorType::OutOfRange,
                                );
                            }
                        }

                        if let Some(max) = field_def.max {
                            if num > max {
                                result.add_field_error(
                                    field_name,
                                    format!("字段 {} 的值不能大于 {}", field_name, max),
                                    ValidationErrorType::OutOfRange,
                                );
                            }
                        }
                    }
                }
                ConfigFieldType::Boolean => {
                    if !value.is_boolean() {
                        result.add_field_error(
                            field_name,
                            format!("字段 {} 应为布尔类型", field_name),
                            ValidationErrorType::TypeMismatch,
                        );
                    }
                }
                ConfigFieldType::Select => {
                    if !value.is_string() {
                        result.add_field_error(
                            field_name,
                            format!("字段 {} 应为字符串类型", field_name),
                            ValidationErrorType::TypeMismatch,
                        );
                    } else if let Some(options) = &field_def.options {
                        let val_str = value.as_str().unwrap();
                        let valid_option = options.iter().any(|opt| opt.value == val_str);

                        if !valid_option {
                            result.add_field_error(
                                field_name,
                                format!(
                                    "字段 {} 的值 '{}' 不在有效选项中",
                                    field_name, val_str
                                ),
                                ValidationErrorType::InvalidOption,
                            );
                        }
                    }
                }
            }
        }

        result
    }

    /// 获取带默认值的完整配置
    ///
    /// 将用户配置与 Schema 默认值合并
    pub async fn get_config_with_defaults(
        &self,
        plugin_id: &str,
        user_config: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let schemas = self.schemas.read().await;
        let mut result = user_config.clone();

        if let Some(schema) = schemas.get(plugin_id) {
            for (field_name, field_def) in schema {
                if !result.contains_key(field_name) {
                    if let Some(default) = &field_def.default {
                        result.insert(field_name.clone(), default.clone());
                    }
                }
            }
        }

        result
    }

    // ========================================================================
    // 配置变更通知 (4.2.4)
    // ========================================================================

    /// 通知配置变更
    ///
    /// 配置更新后，通过事件总线通知相关插件
    pub async fn notify_config_changed(&self, plugin_id: &str, config: &HashMap<String, serde_json::Value>) {
        if let Some(bus) = &self.event_bus {
            let data = serde_json::json!({
                "pluginId": plugin_id,
                "config": config,
            });

            if let Err(e) = bus
                .emit_system(
                    super::event_bus::system_events::PLUGIN_CONFIG_CHANGED,
                    data,
                )
                .await
            {
                log::error!("发送配置变更通知失败: {}", e);
            } else {
                log::debug!("已发送插件 {} 的配置变更通知", plugin_id);
            }
        }
    }

    // ========================================================================
    // 工具方法
    // ========================================================================

    /// 获取插件的敏感字段列表
    pub async fn get_secret_fields(&self, plugin_id: &str) -> Vec<String> {
        self.schemas
            .read()
            .await
            .get(plugin_id)
            .map(|schema| {
                schema
                    .iter()
                    .filter(|(_, field)| field.secret)
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有已注册的插件 ID
    pub async fn get_registered_plugins(&self) -> Vec<String> {
        self.schemas.read().await.keys().cloned().collect()
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> ConfigSchema {
        let mut schema = HashMap::new();

        schema.insert(
            "apiKey".to_string(),
            ConfigField {
                field_type: ConfigFieldType::String,
                required: true,
                secret: true,
                label: Some("API Key".to_string()),
                description: Some("Your API key".to_string()),
                default: None,
                min: None,
                max: None,
                options: None,
            },
        );

        schema.insert(
            "threshold".to_string(),
            ConfigField {
                field_type: ConfigFieldType::Number,
                required: false,
                secret: false,
                label: Some("Threshold".to_string()),
                description: None,
                default: Some(serde_json::json!(80)),
                min: Some(0.0),
                max: Some(100.0),
                options: None,
            },
        );

        schema.insert(
            "enabled".to_string(),
            ConfigField {
                field_type: ConfigFieldType::Boolean,
                required: false,
                secret: false,
                label: Some("Enabled".to_string()),
                description: None,
                default: Some(serde_json::json!(true)),
                min: None,
                max: None,
                options: None,
            },
        );

        schema.insert(
            "mode".to_string(),
            ConfigField {
                field_type: ConfigFieldType::Select,
                required: false,
                secret: false,
                label: Some("Mode".to_string()),
                description: None,
                default: Some(serde_json::json!("auto")),
                min: None,
                max: None,
                options: Some(vec![
                    SelectOption {
                        value: "auto".to_string(),
                        label: "Auto".to_string(),
                    },
                    SelectOption {
                        value: "manual".to_string(),
                        label: "Manual".to_string(),
                    },
                ]),
            },
        );

        schema
    }

    #[tokio::test]
    async fn test_schema_registration() {
        let manager = ConfigManager::new();
        let schema = create_test_schema();

        manager.register_schema("test-plugin", schema.clone()).await;

        let retrieved = manager.get_schema("test-plugin").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_validate_required_field() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        // 缺少必填字段
        let config = HashMap::new();
        let result = manager.validate("test-plugin", &config).await;

        assert!(!result.valid);
        assert_eq!(result.field_errors.len(), 1);
        assert_eq!(result.field_errors[0].field, "apiKey");
        assert_eq!(result.field_errors[0].error_type, ValidationErrorType::Required);
    }

    #[tokio::test]
    async fn test_validate_type_mismatch() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let mut config = HashMap::new();
        config.insert("apiKey".to_string(), serde_json::json!("my-key"));
        config.insert("threshold".to_string(), serde_json::json!("not-a-number"));

        let result = manager.validate("test-plugin", &config).await;

        assert!(!result.valid);
        assert!(result.field_errors.iter().any(|e| e.field == "threshold"));
    }

    #[tokio::test]
    async fn test_validate_out_of_range() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let mut config = HashMap::new();
        config.insert("apiKey".to_string(), serde_json::json!("my-key"));
        config.insert("threshold".to_string(), serde_json::json!(150)); // > max

        let result = manager.validate("test-plugin", &config).await;

        assert!(!result.valid);
        assert!(result.field_errors.iter().any(|e| {
            e.field == "threshold" && e.error_type == ValidationErrorType::OutOfRange
        }));
    }

    #[tokio::test]
    async fn test_validate_invalid_option() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let mut config = HashMap::new();
        config.insert("apiKey".to_string(), serde_json::json!("my-key"));
        config.insert("mode".to_string(), serde_json::json!("unknown"));

        let result = manager.validate("test-plugin", &config).await;

        assert!(!result.valid);
        assert!(result.field_errors.iter().any(|e| {
            e.field == "mode" && e.error_type == ValidationErrorType::InvalidOption
        }));
    }

    #[tokio::test]
    async fn test_validate_success() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let mut config = HashMap::new();
        config.insert("apiKey".to_string(), serde_json::json!("my-key"));
        config.insert("threshold".to_string(), serde_json::json!(75));
        config.insert("enabled".to_string(), serde_json::json!(true));
        config.insert("mode".to_string(), serde_json::json!("auto"));

        let result = manager.validate("test-plugin", &config).await;

        assert!(result.valid);
        assert!(result.field_errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_with_defaults() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let mut config = HashMap::new();
        config.insert("apiKey".to_string(), serde_json::json!("my-key"));

        let full_config = manager.get_config_with_defaults("test-plugin", &config).await;

        assert_eq!(full_config.get("apiKey"), Some(&serde_json::json!("my-key")));
        assert_eq!(full_config.get("threshold"), Some(&serde_json::json!(80)));
        assert_eq!(full_config.get("enabled"), Some(&serde_json::json!(true)));
        assert_eq!(full_config.get("mode"), Some(&serde_json::json!("auto")));
    }

    #[tokio::test]
    async fn test_get_secret_fields() {
        let manager = ConfigManager::new();
        manager.register_schema("test-plugin", create_test_schema()).await;

        let secrets = manager.get_secret_fields("test-plugin").await;

        assert_eq!(secrets, vec!["apiKey"]);
    }

    #[tokio::test]
    async fn test_schema_from_json() {
        let manager = ConfigManager::new();

        let json = serde_json::json!({
            "apiKey": {
                "type": "string",
                "required": true,
                "secret": true,
                "label": "API Key"
            },
            "count": {
                "type": "number",
                "default": 10,
                "min": 1,
                "max": 100
            }
        });

        let result = manager.register_schema_from_json("json-plugin", &json).await;
        assert!(result.is_ok());

        let schema = manager.get_schema("json-plugin").await;
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().len(), 2);
    }
}
