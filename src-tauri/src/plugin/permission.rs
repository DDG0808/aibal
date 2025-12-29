// 权限控制模块
// Phase 4.3: 权限控制
//
// 实现任务:
// - 4.3.1 实现 permissions 声明解析 - 提取权限列表
// - 4.3.2 实现 context.call 权限检查 - 未授权调用被拒绝
// - 4.3.3 实现调用深度限制 - 循环调用被阻止
// - 4.3.4 实现 exposedMethods 注册 - 插件方法可被调用

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ============================================================================
// 权限类型定义
// ============================================================================

/// 权限类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    /// 跨插件调用权限: call:{target_plugin}:{method}
    Call {
        target_plugin: String,
        method: String,
    },
    /// 网络权限
    Network,
    /// 定时器权限
    Timer,
    /// 存储权限
    Storage,
    /// 缓存权限
    Cache,
}

impl Permission {
    /// 从权限字符串解析
    ///
    /// 支持格式:
    /// - "call:{plugin_id}:{method}" -> Call 权限
    /// - "network" -> Network 权限
    /// - "timer" -> Timer 权限
    /// - "storage" -> Storage 权限
    /// - "cache" -> Cache 权限
    pub fn parse(s: &str) -> Option<Self> {
        if s.starts_with("call:") {
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            if parts.len() == 3 && !parts[1].is_empty() && !parts[2].is_empty() {
                return Some(Permission::Call {
                    target_plugin: parts[1].to_string(),
                    method: parts[2].to_string(),
                });
            }
            return None;
        }

        match s.to_lowercase().as_str() {
            "network" | "fetch" => Some(Permission::Network),
            "timer" | "settimeout" => Some(Permission::Timer),
            "storage" => Some(Permission::Storage),
            "cache" => Some(Permission::Cache),
            _ => None,
        }
    }

    /// 转换为权限字符串
    pub fn to_string(&self) -> String {
        match self {
            Permission::Call { target_plugin, method } => {
                format!("call:{}:{}", target_plugin, method)
            }
            Permission::Network => "network".to_string(),
            Permission::Timer => "timer".to_string(),
            Permission::Storage => "storage".to_string(),
            Permission::Cache => "cache".to_string(),
        }
    }
}

// ============================================================================
// 权限检查错误
// ============================================================================

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum PermissionError {
    #[error("权限不足: 插件 {caller} 无权调用 {target}.{method}")]
    PermissionDenied {
        caller: String,
        target: String,
        method: String,
    },

    #[error("调用深度超限: 当前深度 {current}，最大允许 {max}")]
    CallDepthExceeded {
        current: usize,
        max: usize,
    },

    #[error("检测到循环调用: {chain}")]
    CircularCall {
        chain: String,
    },

    #[error("目标插件不存在: {plugin_id}")]
    PluginNotFound {
        plugin_id: String,
    },

    #[error("方法不存在: {plugin_id}.{method}")]
    MethodNotFound {
        plugin_id: String,
        method: String,
    },

    #[error("方法未暴露: {plugin_id}.{method}")]
    MethodNotExposed {
        plugin_id: String,
        method: String,
    },

    #[error("锁争用: {context}")]
    LockContention {
        context: String,
    },
}

// ============================================================================
// 调用栈追踪
// ============================================================================

/// 调用栈
///
/// 追踪跨插件调用链，用于检测循环调用和深度限制
#[derive(Debug, Clone, Default)]
pub struct CallStack {
    /// 调用链 (按顺序存储插件 ID)
    stack: Vec<String>,
    /// 最大深度限制
    max_depth: usize,
}

impl CallStack {
    /// 默认最大调用深度
    pub const DEFAULT_MAX_DEPTH: usize = 3;

    /// 创建新的调用栈
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Vec::new(),
            max_depth,
        }
    }

    /// 使用默认深度创建
    pub fn with_default_depth() -> Self {
        Self::new(Self::DEFAULT_MAX_DEPTH)
    }

    /// 尝试压入调用者
    ///
    /// 检查深度限制和循环调用
    pub fn push(&mut self, plugin_id: &str) -> Result<(), PermissionError> {
        // 检查深度
        if self.stack.len() >= self.max_depth {
            return Err(PermissionError::CallDepthExceeded {
                current: self.stack.len() + 1,
                max: self.max_depth,
            });
        }

        // 检查循环调用
        if self.stack.contains(&plugin_id.to_string()) {
            let mut chain = self.stack.clone();
            chain.push(plugin_id.to_string());
            return Err(PermissionError::CircularCall {
                chain: chain.join(" -> "),
            });
        }

        self.stack.push(plugin_id.to_string());
        Ok(())
    }

    /// 弹出调用者
    pub fn pop(&mut self) -> Option<String> {
        self.stack.pop()
    }

    /// 获取当前深度
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// 获取调用链
    pub fn chain(&self) -> &[String] {
        &self.stack
    }

    /// 获取当前调用者
    pub fn current(&self) -> Option<&str> {
        self.stack.last().map(|s| s.as_str())
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// 获取剩余超时时间 (继承机制)
    ///
    /// 每次调用继承父调用的剩余超时时间
    pub fn remaining_timeout(&self, initial_timeout_ms: u64) -> u64 {
        // 简单实现：每层调用减少一定比例
        // 实际实现应该基于实际消耗的时间计算
        let reduction = self.depth() as u64 * 1000; // 每层减少 1 秒
        initial_timeout_ms.saturating_sub(reduction)
    }
}

// ============================================================================
// 方法注册表
// ============================================================================

/// 暴露的方法信息
#[derive(Debug, Clone)]
pub struct ExposedMethod {
    /// 方法名称
    pub name: String,
    /// 方法描述
    pub description: Option<String>,
    /// 参数 Schema (可选)
    pub params_schema: Option<serde_json::Value>,
    /// 返回值 Schema (可选)
    pub return_schema: Option<serde_json::Value>,
}

/// 方法注册表
///
/// 管理插件暴露的可调用方法
pub struct MethodRegistry {
    /// 方法映射: (plugin_id, method_name) -> ExposedMethod
    pub(crate) methods: RwLock<HashMap<(String, String), ExposedMethod>>,
}

impl MethodRegistry {
    /// 创建新的方法注册表
    pub fn new() -> Self {
        Self {
            methods: RwLock::new(HashMap::new()),
        }
    }

    /// 注册暴露的方法 (4.3.4)
    pub async fn register(
        &self,
        plugin_id: &str,
        method_name: &str,
        description: Option<String>,
    ) {
        let key = (plugin_id.to_string(), method_name.to_string());
        let method = ExposedMethod {
            name: method_name.to_string(),
            description,
            params_schema: None,
            return_schema: None,
        };

        self.methods.write().await.insert(key, method);
        log::debug!("已注册方法: {}.{}", plugin_id, method_name);
    }

    /// 批量注册方法
    pub async fn register_batch(&self, plugin_id: &str, methods: &[String]) {
        let mut registry = self.methods.write().await;

        for method_name in methods {
            let key = (plugin_id.to_string(), method_name.clone());
            let method = ExposedMethod {
                name: method_name.clone(),
                description: None,
                params_schema: None,
                return_schema: None,
            };
            registry.insert(key, method);
        }

        log::debug!("已批量注册插件 {} 的 {} 个方法", plugin_id, methods.len());
    }

    /// 取消注册插件的所有方法
    pub async fn unregister_all(&self, plugin_id: &str) {
        self.methods
            .write()
            .await
            .retain(|(pid, _), _| pid != plugin_id);

        log::debug!("已取消注册插件 {} 的所有方法", plugin_id);
    }

    /// 检查方法是否已注册
    pub async fn is_registered(&self, plugin_id: &str, method_name: &str) -> bool {
        let key = (plugin_id.to_string(), method_name.to_string());
        self.methods.read().await.contains_key(&key)
    }

    /// 获取方法信息
    pub async fn get_method(&self, plugin_id: &str, method_name: &str) -> Option<ExposedMethod> {
        let key = (plugin_id.to_string(), method_name.to_string());
        self.methods.read().await.get(&key).cloned()
    }

    /// 获取插件暴露的所有方法
    pub async fn get_plugin_methods(&self, plugin_id: &str) -> Vec<ExposedMethod> {
        self.methods
            .read()
            .await
            .iter()
            .filter(|((pid, _), _)| pid == plugin_id)
            .map(|(_, method)| method.clone())
            .collect()
    }
}

impl Default for MethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 权限检查器
// ============================================================================

/// 权限检查器
///
/// 管理插件权限声明和运行时权限检查
pub struct PermissionChecker {
    /// 插件权限映射: plugin_id -> Set<Permission>
    permissions: RwLock<HashMap<String, HashSet<Permission>>>,
    /// 方法注册表
    method_registry: Arc<MethodRegistry>,
    /// 调用深度限制
    max_call_depth: usize,
}

impl PermissionChecker {
    /// 创建新的权限检查器
    pub fn new(method_registry: Arc<MethodRegistry>) -> Self {
        Self {
            permissions: RwLock::new(HashMap::new()),
            method_registry,
            max_call_depth: CallStack::DEFAULT_MAX_DEPTH,
        }
    }

    /// 设置最大调用深度
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_call_depth = depth;
        self
    }

    // ========================================================================
    // 权限注册 (4.3.1)
    // ========================================================================

    /// 注册插件的权限声明
    ///
    /// 从 manifest.permissions 解析权限列表
    pub async fn register_permissions(&self, plugin_id: &str, permission_strs: &[String]) {
        let mut parsed_permissions = HashSet::new();

        for perm_str in permission_strs {
            match Permission::parse(perm_str) {
                Some(perm) => {
                    parsed_permissions.insert(perm);
                }
                None => {
                    log::warn!("插件 {} 声明了无效的权限: {}", plugin_id, perm_str);
                }
            }
        }

        if !parsed_permissions.is_empty() {
            log::debug!(
                "已注册插件 {} 的 {} 个权限",
                plugin_id,
                parsed_permissions.len()
            );
        }

        self.permissions
            .write()
            .await
            .insert(plugin_id.to_string(), parsed_permissions);
    }

    /// 取消注册插件的权限
    pub async fn unregister_permissions(&self, plugin_id: &str) {
        self.permissions.write().await.remove(plugin_id);
        log::debug!("已取消注册插件 {} 的权限", plugin_id);
    }

    // ========================================================================
    // 权限检查 (4.3.2)
    // ========================================================================

    /// 检查插件是否有指定权限
    pub async fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
        self.permissions
            .read()
            .await
            .get(plugin_id)
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }

    /// 检查跨插件调用权限
    ///
    /// 验证调用方是否有权调用目标插件的指定方法
    pub async fn check_call_permission(
        &self,
        caller: &str,
        target: &str,
        method: &str,
    ) -> Result<(), PermissionError> {
        // 1. 检查调用权限声明
        let required_permission = Permission::Call {
            target_plugin: target.to_string(),
            method: method.to_string(),
        };

        if !self.has_permission(caller, &required_permission).await {
            return Err(PermissionError::PermissionDenied {
                caller: caller.to_string(),
                target: target.to_string(),
                method: method.to_string(),
            });
        }

        // 2. 检查目标方法是否已暴露
        if !self.method_registry.is_registered(target, method).await {
            return Err(PermissionError::MethodNotExposed {
                plugin_id: target.to_string(),
                method: method.to_string(),
            });
        }

        Ok(())
    }

    /// 检查跨插件调用权限（同步版本，用于 JS 回调）
    ///
    /// 使用 try_read + 重试机制，避免阻塞运行时线程
    /// 如 JS 函数回调中的 context.call()
    ///
    /// 注意：如果锁争用严重，可能返回 LockContention 错误
    pub fn check_call_permission_sync(
        &self,
        caller: &str,
        target: &str,
        method: &str,
    ) -> Result<(), PermissionError> {
        const MAX_RETRIES: u32 = 5;

        // 1. 检查调用权限声明
        let required_permission = Permission::Call {
            target_plugin: target.to_string(),
            method: method.to_string(),
        };

        // 使用 try_read + 重试获取锁，避免阻塞运行时
        let has_perm = {
            let mut result = None;
            for _ in 0..MAX_RETRIES {
                if let Ok(guard) = self.permissions.try_read() {
                    result = Some(
                        guard
                            .get(caller)
                            .map(|perms| perms.contains(&required_permission))
                            .unwrap_or(false),
                    );
                    break;
                }
                std::thread::yield_now();
            }
            result.ok_or_else(|| PermissionError::LockContention {
                context: format!("检查 {} 的调用权限", caller),
            })?
        };

        if !has_perm {
            return Err(PermissionError::PermissionDenied {
                caller: caller.to_string(),
                target: target.to_string(),
                method: method.to_string(),
            });
        }

        // 2. 检查目标方法是否已暴露
        let is_registered = {
            let mut result = None;
            for _ in 0..MAX_RETRIES {
                if let Ok(guard) = self.method_registry.methods.try_read() {
                    result = Some(guard.contains_key(&(target.to_string(), method.to_string())));
                    break;
                }
                std::thread::yield_now();
            }
            result.ok_or_else(|| PermissionError::LockContention {
                context: format!("检查方法 {}::{} 是否已暴露", target, method),
            })?
        };

        if !is_registered {
            return Err(PermissionError::MethodNotExposed {
                plugin_id: target.to_string(),
                method: method.to_string(),
            });
        }

        Ok(())
    }

    // ========================================================================
    // 调用深度检查 (4.3.3)
    // ========================================================================

    /// 创建新的调用栈
    pub fn create_call_stack(&self) -> CallStack {
        CallStack::new(self.max_call_depth)
    }

    /// 检查调用深度
    pub fn check_call_depth(&self, stack: &CallStack, target: &str) -> Result<(), PermissionError> {
        // 创建临时栈进行检查
        let mut temp_stack = stack.clone();
        temp_stack.push(target)
    }

    // ========================================================================
    // 完整调用检查
    // ========================================================================

    /// 执行完整的调用前检查
    ///
    /// 包括权限检查和深度检查
    pub async fn validate_call(
        &self,
        caller: &str,
        target: &str,
        method: &str,
        stack: &CallStack,
    ) -> Result<(), PermissionError> {
        // 1. 权限检查
        self.check_call_permission(caller, target, method).await?;

        // 2. 深度检查
        self.check_call_depth(stack, target)?;

        Ok(())
    }

    // ========================================================================
    // 工具方法
    // ========================================================================

    /// 获取插件的所有权限
    pub async fn get_plugin_permissions(&self, plugin_id: &str) -> Vec<String> {
        self.permissions
            .read()
            .await
            .get(plugin_id)
            .map(|perms| perms.iter().map(|p| p.to_string()).collect())
            .unwrap_or_default()
    }

    /// 获取方法注册表引用
    pub fn method_registry(&self) -> &Arc<MethodRegistry> {
        &self.method_registry
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_parse_call() {
        let perm = Permission::parse("call:notifications:send");
        assert!(perm.is_some());

        match perm.unwrap() {
            Permission::Call { target_plugin, method } => {
                assert_eq!(target_plugin, "notifications");
                assert_eq!(method, "send");
            }
            _ => panic!("Expected Call permission"),
        }
    }

    #[test]
    fn test_permission_parse_network() {
        assert_eq!(Permission::parse("network"), Some(Permission::Network));
        assert_eq!(Permission::parse("fetch"), Some(Permission::Network));
    }

    #[test]
    fn test_permission_parse_invalid() {
        assert!(Permission::parse("call:").is_none());
        assert!(Permission::parse("call:plugin:").is_none());
        assert!(Permission::parse("unknown").is_none());
    }

    #[test]
    fn test_call_stack_depth() {
        let mut stack = CallStack::new(3);

        assert!(stack.push("plugin-a").is_ok());
        assert!(stack.push("plugin-b").is_ok());
        assert!(stack.push("plugin-c").is_ok());

        // 第 4 次应该失败
        let result = stack.push("plugin-d");
        assert!(matches!(result, Err(PermissionError::CallDepthExceeded { .. })));
    }

    #[test]
    fn test_call_stack_circular() {
        let mut stack = CallStack::new(10);

        assert!(stack.push("plugin-a").is_ok());
        assert!(stack.push("plugin-b").is_ok());

        // 循环调用应该失败
        let result = stack.push("plugin-a");
        assert!(matches!(result, Err(PermissionError::CircularCall { .. })));
    }

    #[test]
    fn test_call_stack_pop() {
        let mut stack = CallStack::new(3);

        stack.push("plugin-a").unwrap();
        stack.push("plugin-b").unwrap();

        assert_eq!(stack.pop(), Some("plugin-b".to_string()));
        assert_eq!(stack.depth(), 1);

        // 弹出后应该可以再次压入
        assert!(stack.push("plugin-b").is_ok());
    }

    #[tokio::test]
    async fn test_method_registry() {
        let registry = MethodRegistry::new();

        registry.register("notifications", "send", Some("发送通知".to_string())).await;

        assert!(registry.is_registered("notifications", "send").await);
        assert!(!registry.is_registered("notifications", "other").await);

        let method = registry.get_method("notifications", "send").await;
        assert!(method.is_some());
        assert_eq!(method.unwrap().description, Some("发送通知".to_string()));
    }

    #[tokio::test]
    async fn test_permission_checker() {
        let registry = Arc::new(MethodRegistry::new());
        let checker = PermissionChecker::new(registry.clone());

        // 注册权限
        checker
            .register_permissions(
                "claude-usage",
                &["call:notifications:send".to_string()],
            )
            .await;

        // 注册方法
        registry.register("notifications", "send", None).await;

        // 应该通过
        let result = checker
            .check_call_permission("claude-usage", "notifications", "send")
            .await;
        assert!(result.is_ok());

        // 未授权应该失败
        let result = checker
            .check_call_permission("other-plugin", "notifications", "send")
            .await;
        assert!(matches!(result, Err(PermissionError::PermissionDenied { .. })));
    }

    #[tokio::test]
    async fn test_validate_call() {
        let registry = Arc::new(MethodRegistry::new());
        let checker = PermissionChecker::new(registry.clone());

        // 注册权限和方法
        checker
            .register_permissions("plugin-a", &["call:plugin-b:action".to_string()])
            .await;
        registry.register("plugin-b", "action", None).await;

        // 创建调用栈
        let mut stack = checker.create_call_stack();
        stack.push("plugin-a").unwrap();

        // 应该通过
        let result = checker.validate_call("plugin-a", "plugin-b", "action", &stack).await;
        assert!(result.is_ok());
    }
}
