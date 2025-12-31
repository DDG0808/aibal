// 插件系统模块
// Phase 2: 插件运行时核心
// Phase 4: 通信与配置
// Phase 6: 监控层

pub mod config;
pub mod event_bus;
pub mod lifecycle;
pub mod monitoring;
pub mod permission;
pub mod runtime;
pub mod sandbox;
pub mod types;
pub mod watcher;

#[cfg(test)]
mod tests;

// 导出运行时类型
pub use runtime::{
    Executor, InterruptController, PluginExecutor, RuntimeError, SandboxConfig, SandboxRuntime,
    Watchdog, DEFAULT_EXECUTION_TIMEOUT, DEFAULT_MEMORY_LIMIT, DEFAULT_STACK_SIZE,
};

// 导出沙盒 API
pub use sandbox::{
    ConsoleApi, EncodingApi, FetchApi, PluginErrorApi, RequestManager, SandboxApiInitializer,
    TimerApi, TimerRegistry, UrlSecurityChecker,
};

// 导出生命周期管理
pub use lifecycle::{
    LifecycleError, PluginDiscovery, PluginInstance, PluginManager, PluginManifest,
    PluginState, ResourceRegistry, ResourceType,
};

// 导出热重载
pub use watcher::{HotReloadEvent, HotReloadManager, PluginWatcher};

// 导出监控层 (Phase 6)
pub use monitoring::{
    Alert, AlertData, AlertManager, AlertSeverity, AlertStats, AlertThresholds, AlertType,
    NotificationHandler, SlidingWindow, TauriNotificationHandler, WindowStats,
    create_alert_manager_with_notifications, DEFAULT_WINDOW_SIZE,
};

// 导出类型定义
pub use types::*;

// 导出事件总线 (Phase 4.1)
pub use event_bus::{
    EventBus, EventBusConfig, EventBusError, EventBusStats, EventDispatchResult, EventPrefix,
    QueuedEvent, system_events,
};

// 导出配置管理 (Phase 4.2)
pub use config::{
    ConfigField, ConfigFieldType, ConfigManager, ConfigSchema, ConfigValidationResult,
    FieldValidationError, SelectOption, ValidationErrorType,
};

// 导出权限控制 (Phase 4.3)
pub use permission::{
    CallStack, ExposedMethod, MethodRegistry, Permission, PermissionChecker, PermissionError,
};
