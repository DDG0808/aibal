// 沙盒 API 模块
// Phase 2.2: 沙盒安全层
// Phase 4: 通信与配置
//
// 提供给 JS 插件使用的安全 API

pub mod console;
pub mod context;
pub mod encoding;
pub mod error;
pub mod fetch;
pub mod timer;

// 导出所有沙盒 API
pub use console::ConsoleApi;
pub use context::{EmitRequest, PluginCallRequest, PluginContextApi, PluginContextConfig};
pub use encoding::EncodingApi;
pub use error::PluginErrorApi;
pub use fetch::{FetchApi, RequestManager, UrlSecurityChecker};
pub use timer::{TimerApi, TimerRegistry};

use std::sync::Arc;
use rquickjs::{AsyncContext, Result as JsResult};

/// 沙盒 API 初始化器
/// 负责向 JS 上下文注入所有安全 API
pub struct SandboxApiInitializer;

impl SandboxApiInitializer {
    /// 初始化核心沙盒 API（内部方法）
    ///
    /// 注入基础 API：console、encoding、error
    fn inject_core_apis(ctx: &rquickjs::Ctx<'_>) -> JsResult<()> {
        // 注入 console API
        ConsoleApi::inject(ctx)?;

        // 注入编码 API
        EncodingApi::inject(ctx)?;

        // 注入 PluginError 类
        PluginErrorApi::inject(ctx)?;

        Ok(())
    }

    /// 移除危险的全局对象并冻结相关原型链
    ///
    /// 安全策略（基于最佳实践）：
    /// 1. 删除 eval 和 Function 全局对象
    /// 2. 若删除失败，覆盖为抛错 accessor（防止 QuickJS 属性不可配置的情况）
    /// 3. 冻结所有函数构造器原型链：Function、AsyncFunction、GeneratorFunction、AsyncGeneratorFunction
    ///
    /// 参考：https://portswigger.net/research/attacking-and-defending-javascript-sandboxes
    fn remove_dangerous_globals(ctx: &rquickjs::Ctx<'_>) -> JsResult<()> {
        // 彻底禁用危险全局对象和所有函数构造器
        // 使用单一 JS 代码块确保原子性和完整性
        // 注意：QuickJS 对某些 ES2018+ 特性支持有限，需要优雅降级
        ctx.eval::<(), _>(r#"
            (function() {
                'use strict';

                // 辅助函数：将属性替换为抛错 accessor
                function disableProperty(obj, prop, msg) {
                    try {
                        Object.defineProperty(obj, prop, {
                            get: function() { throw new TypeError(msg); },
                            set: function() { throw new TypeError(msg); },
                            configurable: false
                        });
                        return true;
                    } catch(e) {
                        // 属性可能已经是不可配置的，尝试删除
                        try { delete obj[prop]; return true; } catch(e2) { return false; }
                    }
                }

                // 辅助函数：禁用构造器原型链
                function disableConstructor(constructorFn, name) {
                    if (!constructorFn || !constructorFn.prototype) return false;
                    return disableProperty(
                        constructorFn.prototype,
                        'constructor',
                        name + ' constructor is disabled in sandbox'
                    );
                }

                // 辅助函数：检查属性是否被禁用（抛出任何错误都算成功）
                function isDisabled(obj, prop) {
                    try {
                        var val = obj[prop];
                        return val === undefined;  // undefined 也算禁用成功
                    } catch(e) {
                        return true;  // 抛出任何错误都算成功
                    }
                }

                // 1. 移除/禁用 eval
                if (typeof globalThis.eval !== 'undefined') {
                    try {
                        delete globalThis.eval;
                    } catch(e) {}
                    disableProperty(globalThis, 'eval', 'eval is disabled in sandbox');
                }

                // 2. 移除/禁用 Function
                if (typeof globalThis.Function !== 'undefined') {
                    try {
                        delete globalThis.Function;
                    } catch(e) {}
                    disableProperty(globalThis, 'Function', 'Function constructor is disabled in sandbox');
                }

                // 3. 冻结 Function.prototype.constructor（通过任意函数获取）
                var FunctionConstructor = (function(){}).constructor;
                disableConstructor(FunctionConstructor, 'Function');

                // 4. 冻结 AsyncFunction.prototype.constructor
                try {
                    var AsyncFunctionConstructor = (async function(){}).constructor;
                    disableConstructor(AsyncFunctionConstructor, 'AsyncFunction');
                } catch(e) {
                    // QuickJS 可能不支持 async function，静默跳过
                }

                // 5. 冻结 GeneratorFunction.prototype.constructor
                try {
                    var GeneratorFunctionConstructor = (function*(){}).constructor;
                    disableConstructor(GeneratorFunctionConstructor, 'GeneratorFunction');
                } catch(e) {
                    // QuickJS 可能不支持 generator，静默跳过
                }

                // 6. 冻结 AsyncGeneratorFunction.prototype.constructor
                // 这是 ECMAScript 2018+ 的特性，QuickJS 可能不支持
                try {
                    var AsyncGeneratorFunctionConstructor = (async function*(){}).constructor;
                    disableConstructor(AsyncGeneratorFunctionConstructor, 'AsyncGeneratorFunction');
                } catch(e) {
                    // 预期：QuickJS 可能不支持 async generator，静默跳过
                }

                // 6.5 禁用 WebAssembly（另一个动态代码执行入口）
                if (typeof globalThis.WebAssembly !== 'undefined') {
                    try {
                        delete globalThis.WebAssembly;
                    } catch(e) {}
                    disableProperty(globalThis, 'WebAssembly', 'WebAssembly is disabled in sandbox');
                }

                // 7. 额外安全：冻结 Object.prototype 防止原型污染
                // 注意：这可能影响某些插件的正常运行，改为可选
                // try {
                //     Object.freeze(Object.prototype);
                // } catch(e) {}

                // 8. 冻结 Function.prototype 防止通过原型链恢复构造器
                try {
                    Object.freeze(FunctionConstructor.prototype);
                } catch(e) {}

                // ============================================================
                // 9. 自检：验证核心禁用是否成功
                // 只检查关键项，使用宽松验证（任何错误都算成功）
                // ============================================================
                var errors = [];

                // 检查 eval 是否被禁用
                if (!isDisabled(globalThis, 'eval')) {
                    // 再次尝试检查 eval 是否可调用
                    try {
                        var testEval = globalThis.eval;
                        if (typeof testEval === 'function') {
                            errors.push('eval is still accessible');
                        }
                    } catch(e) {
                        // 抛出错误说明已禁用，正常
                    }
                }

                // 检查 Function 是否被禁用
                if (!isDisabled(globalThis, 'Function')) {
                    try {
                        var testFunc = globalThis.Function;
                        if (typeof testFunc === 'function') {
                            errors.push('Function is still accessible');
                        }
                    } catch(e) {
                        // 抛出错误说明已禁用，正常
                    }
                }

                // 检查 Function.prototype.constructor 是否被禁用
                // 这是最关键的检查，因为这是绕过沙盒的主要途径
                try {
                    var fc = FunctionConstructor.prototype.constructor;
                    if (typeof fc === 'function') {
                        errors.push('Function.prototype.constructor is still accessible');
                    }
                } catch(e) {
                    // 抛出错误说明已禁用，正常
                }

                // fail-close：若有任何核心错误，抛出异常让沙盒初始化失败
                if (errors.length > 0) {
                    throw new Error('Sandbox security check failed: ' + errors.join('; '));
                }
            })();
        "#).map_err(|e| {
            log::error!("冻结函数构造器失败（严重安全问题）: {}", e);
            e
        })?;

        log::debug!("已禁用危险全局对象并冻结所有函数构造器原型链");
        Ok(())
    }

    /// 初始化所有沙盒 API（不含需要外部依赖的 API）
    ///
    /// 包含：console、encoding、error
    /// 不包含：fetch（需要 RequestManager）、timer（需要 TimerRegistry）
    pub async fn init_basic(ctx: &AsyncContext) -> JsResult<()> {
        ctx.with(|ctx| {
            Self::inject_core_apis(&ctx)?;
            Self::remove_dangerous_globals(&ctx)?;
            Ok(())
        })
        .await
    }

    /// 初始化所有沙盒 API（包含 fetch API）
    ///
    /// 在 init_basic 基础上额外注入安全的 fetch API。
    /// fetch 需要 RequestManager 来管理并发请求和安全检查。
    ///
    /// # 注意
    /// 当前 fetch API 处于开发中状态，调用时会抛出错误而非执行网络请求。
    /// 这是设计决策，避免因异步集成未完成而导致虚假安全感。
    pub async fn init_with_fetch(
        ctx: &AsyncContext,
        request_manager: Arc<RequestManager>,
    ) -> JsResult<()> {
        ctx.with(|ctx| {
            // 注入核心 API
            Self::inject_core_apis(&ctx)?;

            // 注入 fetch API（当前为错误抛出模式）
            FetchApi::inject(&ctx, request_manager)?;

            // 移除危险全局对象
            Self::remove_dangerous_globals(&ctx)?;

            Ok(())
        })
        .await
    }

    /// 初始化沙盒 API（包含 plugin context API）
    ///
    /// Phase 4: 通信与配置
    /// 在 init_basic 基础上额外注入 plugin context API:
    /// - context.pluginId - 当前插件 ID
    /// - context.config - 插件配置
    /// - context.emit(event, data) - 发布事件
    /// - context.call(pluginId, method, params) - 跨插件调用
    /// - context.log(level, message) - 带前缀日志
    ///
    /// # 参数
    /// - `ctx`: QuickJS 上下文
    /// - `context_config`: 插件上下文配置
    pub async fn init_with_plugin_context(
        ctx: &AsyncContext,
        context_config: PluginContextConfig,
    ) -> JsResult<()> {
        ctx.with(|ctx| {
            // 注入核心 API
            Self::inject_core_apis(&ctx)?;

            // 注入 plugin context API (Phase 4)
            PluginContextApi::inject(&ctx, context_config)?;

            // 移除危险全局对象
            Self::remove_dangerous_globals(&ctx)?;

            Ok(())
        })
        .await
    }

    /// 初始化完整沙盒 API（包含所有可选组件）
    ///
    /// Phase 4: 通信与配置
    /// 包含：
    /// - 核心 API（console、encoding、error）
    /// - fetch API（网络请求）
    /// - timer API（定时器）
    /// - plugin context API（事件、调用、配置）
    ///
    /// # 参数
    /// - `ctx`: QuickJS 上下文
    /// - `context_config`: 插件上下文配置
    /// - `request_manager`: 请求管理器（用于 fetch）
    /// - `timer_registry`: 定时器注册表（用于 timer）
    pub async fn init_full(
        ctx: &AsyncContext,
        context_config: PluginContextConfig,
        request_manager: Option<Arc<RequestManager>>,
        timer_registry: Option<Arc<TimerRegistry>>,
    ) -> JsResult<()> {
        ctx.with(|ctx| {
            // 注入核心 API
            Self::inject_core_apis(&ctx)?;

            // 注入 plugin context API (Phase 4)
            PluginContextApi::inject(&ctx, context_config)?;

            // 注入 fetch API（如果有权限）
            if let Some(rm) = request_manager {
                FetchApi::inject(&ctx, rm)?;
            }

            // 注入 timer API（如果有权限）
            if let Some(tr) = timer_registry {
                TimerApi::inject(&ctx, tr)?;
            }

            // 移除危险全局对象
            Self::remove_dangerous_globals(&ctx)?;

            Ok(())
        })
        .await
    }
}
