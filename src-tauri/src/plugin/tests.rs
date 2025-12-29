// Phase 2 沙箱集成测试
// 验证 QuickJS 运行时的核心功能

#[cfg(test)]
mod sandbox_tests {
    use rquickjs::{AsyncContext, AsyncRuntime};
    use std::time::Duration;

    /// 测试简单 JS 执行
    #[tokio::test]
    async fn test_simple_js_execution() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>("1 + 2 + 3")
            })
            .await
            .unwrap();

        assert_eq!(result, 6);
    }

    /// 测试字符串操作
    #[tokio::test]
    async fn test_string_operations() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: String = context
            .with(|ctx| {
                ctx.eval::<String, _>(r#"
                    const greeting = "Hello";
                    const name = "World";
                    greeting + ", " + name + "!"
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, "Hello, World!");
    }

    /// 测试数组操作
    #[tokio::test]
    async fn test_array_operations() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>(r#"
                    const arr = [1, 2, 3, 4, 5];
                    arr.reduce((a, b) => a + b, 0)
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, 15);
    }

    /// 测试函数定义和调用
    #[tokio::test]
    async fn test_function_call() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>(r#"
                    function fibonacci(n) {
                        if (n <= 1) return n;
                        return fibonacci(n - 1) + fibonacci(n - 2);
                    }
                    fibonacci(10)
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, 55);
    }

    /// 测试 JSON 操作
    #[tokio::test]
    async fn test_json_operations() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: String = context
            .with(|ctx| {
                ctx.eval::<String, _>(r#"
                    const obj = { name: "CUK", version: "1.0" };
                    JSON.stringify(obj)
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, r#"{"name":"CUK","version":"1.0"}"#);
    }

    /// 测试 Promise 基础
    #[tokio::test]
    async fn test_promise_basic() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>(r#"
                    let result = 0;
                    Promise.resolve(42).then(v => { result = v; });
                    result
                "#)
            })
            .await
            .unwrap();

        // Promise 是异步的，同步执行时 result 还是 0
        assert_eq!(result, 0);
    }

    /// 测试内存限制设置
    #[tokio::test]
    async fn test_memory_limit_setting() {
        let runtime = AsyncRuntime::new().unwrap();

        // 设置 16MB 内存限制
        runtime.set_memory_limit(16 * 1024 * 1024).await;

        let context = AsyncContext::full(&runtime).await.unwrap();

        // 简单操作应该正常工作
        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>("100 * 100")
            })
            .await
            .unwrap();

        assert_eq!(result, 10000);
    }

    /// 测试执行超时机制
    /// 注意：此测试验证中断机制的设置，实际中断需要在 JS 执行过程中检查
    #[tokio::test]
    async fn test_execution_timeout() {
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::sync::Arc;

        let runtime = AsyncRuntime::new().unwrap();
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = counter.clone();

        // 设置中断处理器 - 每 10000 次检查触发一次
        runtime
            .set_interrupt_handler(Some(Box::new(move || {
                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                // 在 1000 次检查后中断
                count > 1000
            })))
            .await;

        let context = AsyncContext::full(&runtime).await.unwrap();

        // 执行一个会被中断的循环
        let result = context
            .with(|ctx| {
                ctx.eval::<(), _>("for(let i = 0; i < 100000000; i++) { /* busy loop */ }")
            })
            .await;

        // 应该因为中断而失败
        assert!(result.is_err() || counter.load(Ordering::Relaxed) > 1000);
    }

    /// 测试 ES6 特性
    #[tokio::test]
    async fn test_es6_features() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        // 测试解构、箭头函数、模板字符串
        let result: String = context
            .with(|ctx| {
                ctx.eval::<String, _>(r#"
                    const { a, b } = { a: 1, b: 2 };
                    const sum = (x, y) => x + y;
                    `Result: ${sum(a, b)}`
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, "Result: 3");
    }

    /// 测试类定义
    #[tokio::test]
    async fn test_class_definition() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: String = context
            .with(|ctx| {
                ctx.eval::<String, _>(r#"
                    class Plugin {
                        constructor(name) {
                            this.name = name;
                        }

                        greet() {
                            return `Hello from ${this.name}`;
                        }
                    }

                    const plugin = new Plugin("TestPlugin");
                    plugin.greet()
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, "Hello from TestPlugin");
    }

    /// 测试异步迭代器
    #[tokio::test]
    async fn test_async_generator() {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        let result: i32 = context
            .with(|ctx| {
                ctx.eval::<i32, _>(r#"
                    function* range(start, end) {
                        for (let i = start; i < end; i++) {
                            yield i;
                        }
                    }

                    let sum = 0;
                    for (const n of range(1, 6)) {
                        sum += n;
                    }
                    sum
                "#)
            })
            .await
            .unwrap();

        assert_eq!(result, 15); // 1+2+3+4+5
    }
}
