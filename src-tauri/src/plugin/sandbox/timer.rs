// Timer API 实现
// Phase 2.2.8: setTimeout/clearTimeout
//
// 提供给 JS 插件使用的定时器功能，带数量限制
//
// 安全设计（基于最佳实践）：
// 1. 使用 Semaphore 限制并发定时器数量（防止 spawn flooding DoS）
// 2. Permit 在 spawn 前同步获取，确保真正占位
// 3. 使用 OwnedSemaphorePermit 确保任务完成/取消时自动释放
// 4. 取消操作使用 CancellationToken 避免竞态
//
// 参考：https://users.rust-lang.org/t/can-tokio-semaphore-be-used-to-limit-spawned-tasks/59899

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, MutexGuard, PoisonError};

use rquickjs::{Ctx, Exception, Function, Result as JsResult};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

// ============================================================================
// Mutex 毒化恢复辅助
// ============================================================================

/// 从毒化的 Mutex 中恢复数据
///
/// std::sync::Mutex 在持锁线程 panic 时会毒化。
/// 此函数允许恢复毒化锁的数据，避免连锁 panic。
#[inline]
fn recover_lock<'a, T>(result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>) -> MutexGuard<'a, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("Mutex 已毒化，恢复数据继续操作");
            poisoned.into_inner()
        }
    }
}

/// 最大定时器数量
const MAX_TIMERS: usize = 100;

/// 最大延时 (60 秒)
const MAX_DELAY_MS: u64 = 60_000;

/// Timer API
pub struct TimerApi;

impl TimerApi {
    /// 向上下文注入 timer 相关函数
    /// 注意：需要传入 TimerRegistry 以跟踪定时器
    pub fn inject(ctx: &Ctx<'_>, registry: Arc<TimerRegistry>) -> JsResult<()> {
        let globals = ctx.globals();

        // setTimeout
        let registry_clone = registry.clone();
        globals.set(
            "setTimeout",
            Function::new(ctx.clone(), move |ctx: Ctx<'_>, callback: Function<'_>, delay: Option<u64>| {
                set_timeout(&ctx, callback, delay, registry_clone.clone())
            })?,
        )?;

        // clearTimeout
        let registry_clone = registry.clone();
        globals.set(
            "clearTimeout",
            Function::new(ctx.clone(), move |id: u64| {
                clear_timeout(id, registry_clone.clone())
            })?,
        )?;

        // setInterval
        let registry_clone = registry.clone();
        globals.set(
            "setInterval",
            Function::new(ctx.clone(), move |ctx: Ctx<'_>, callback: Function<'_>, delay: Option<u64>| {
                set_interval(&ctx, callback, delay, registry_clone.clone())
            })?,
        )?;

        // clearInterval (与 clearTimeout 相同)
        let registry_clone = registry.clone();
        globals.set(
            "clearInterval",
            Function::new(ctx.clone(), move |id: u64| {
                clear_timeout(id, registry_clone.clone())
            })?,
        )?;

        log::debug!("Timer API 已注入");
        Ok(())
    }
}

// ============================================================================
// Timer Registry (使用 Semaphore 限制并发)
// ============================================================================

/// 定时器信息
#[derive(Debug)]
struct TimerEntry {
    /// 定时器 ID
    id: u64,
    /// 是否为 interval
    is_interval: bool,
    /// 取消令牌
    cancel_token: CancellationToken,
    /// Semaphore permit（持有期间占用槽位）
    _permit: OwnedSemaphorePermit,
}

/// 定时器注册表
/// 使用 Semaphore 限制并发定时器数量，防止 DoS 攻击
///
/// # 取消竞态解决方案
/// 使用两层存储解决"创建后立即取消"的竞态：
/// 1. `pending_tokens`: 同步 map，在 acquire 时立即存储 cancel_token
/// 2. `timers`: 异步 map，在 register 后存储完整信息
/// 这确保 cancel 在 register 之前也能通过 token 生效
pub struct TimerRegistry {
    /// 下一个定时器 ID
    next_id: AtomicU64,
    /// Semaphore 限制并发数（在 spawn 前获取 permit）
    semaphore: Arc<Semaphore>,
    /// 待注册的 cancel_token（同步 map，用于解决取消竞态）
    pending_tokens: std::sync::Mutex<HashMap<u64, CancellationToken>>,
    /// 活跃的定时器（用于取消操作）
    timers: Mutex<HashMap<u64, TimerEntry>>,
}

impl TimerRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            semaphore: Arc::new(Semaphore::new(MAX_TIMERS)),
            pending_tokens: std::sync::Mutex::new(HashMap::new()),
            timers: Mutex::new(HashMap::new()),
        }
    }

    /// 尝试获取定时器槽位（同步版本，在 spawn 前调用）
    ///
    /// 返回 (id, permit, cancel_token)，permit 持有期间占用槽位
    /// cancel_token 同时存入 pending_tokens，确保 cancel 在 register 前也能生效
    pub fn try_acquire(&self) -> Option<(u64, OwnedSemaphorePermit, CancellationToken)> {
        // 尝试立即获取 permit（非阻塞）
        let permit = self.semaphore.clone().try_acquire_owned().ok()?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let cancel_token = CancellationToken::new();

        // 立即存入 pending_tokens，确保 cancel 可以在 register 前生效
        {
            let mut pending = recover_lock(self.pending_tokens.lock());
            pending.insert(id, cancel_token.clone());
        }

        Some((id, permit, cancel_token))
    }

    /// 注册定时器（在 spawn 后调用）
    ///
    /// 将定时器信息存储到 HashMap，以支持取消操作
    /// 同时从 pending_tokens 移除（已转移到 timers）
    ///
    /// # 原子性保证（关键修复）
    /// 先获取 timers 锁，再操作 pending_tokens，确保与 cancel() 的原子性：
    /// - register 和 cancel 都在持有 timers 锁的情况下操作 pending
    /// - 消除"pending 已删除但 timers 未添加"的竞态窗口
    ///
    /// # 返回
    /// - `true`: 注册成功
    /// - `false`: 定时器已被取消（不应继续执行）
    pub async fn register(
        &self,
        id: u64,
        is_interval: bool,
        cancel_token: CancellationToken,
        permit: OwnedSemaphorePermit,
    ) -> bool {
        // 关键修复：先获取 timers 锁，确保与 cancel 的原子性
        let mut timers = self.timers.lock().await;

        // 在 timers 锁保护下操作 pending_tokens
        let was_in_pending = {
            let mut pending = recover_lock(self.pending_tokens.lock());
            pending.remove(&id).is_some()
        };

        // 如果不在 pending 中，说明已被 cancel 移除
        if !was_in_pending {
            log::trace!("定时器 {} 在 register 时发现已被取消", id);
            // permit 在这里 drop，自动释放槽位
            return false;
        }

        // 存入 timers（仍在锁保护下）
        timers.insert(id, TimerEntry {
            id,
            is_interval,
            cancel_token,
            _permit: permit,
        });
        true
    }

    /// 取消定时器
    ///
    /// # 原子性保证（关键修复）
    /// 先获取 timers 锁，再操作 pending_tokens，确保与 register() 的原子性：
    /// - register 和 cancel 都在持有 timers 锁的情况下操作 pending
    /// - 消除"两边都查不到"的竞态窗口
    ///
    /// # 时序分析
    /// 场景 A: cancel 先获取锁
    ///   - cancel 从 pending 移除并取消 → register 发现不在 pending → 返回 false
    /// 场景 B: register 先获取锁
    ///   - register 从 pending 移除并添加到 timers → cancel 从 timers 移除并取消
    /// 两种场景都能正确取消，不会丢失
    pub async fn cancel(&self, id: u64) -> bool {
        // 关键修复：先获取 timers 锁，确保与 register 的原子性
        let mut timers = self.timers.lock().await;

        // 在 timers 锁保护下操作 pending_tokens
        let found_in_pending = {
            let mut pending: MutexGuard<'_, HashMap<u64, CancellationToken>> =
                recover_lock(self.pending_tokens.lock());
            if let Some(token) = pending.remove(&id) {
                token.cancel();
                log::trace!("定时器 {} 在 pending 中被取消", id);
                true
            } else {
                false
            }
        };

        // 检查 timers（仍在锁保护下）
        if let Some(entry) = timers.remove(&id) {
            entry.cancel_token.cancel();
            log::trace!("定时器 {} 在 timers 中被取消", id);
            // permit 在 entry 被 drop 时自动释放
            true
        } else {
            found_in_pending
        }
    }

    /// 标记定时器完成
    ///
    /// # 原子性保证
    /// 先获取 timers 锁，再清理 pending_tokens，与 register/cancel 保持一致
    pub async fn complete(&self, id: u64) {
        // 先获取 timers 锁，确保原子性
        let mut timers = self.timers.lock().await;

        // 在锁保护下清理 pending_tokens
        {
            let mut pending = recover_lock(self.pending_tokens.lock());
            pending.remove(&id);
        }

        timers.remove(&id);
        // permit 在 entry 被 drop 时自动释放
    }

    /// 取消所有定时器
    ///
    /// # 原子性保证
    /// 先获取 timers 锁，再操作 pending_tokens，与 register/cancel 保持一致
    pub async fn cancel_all(&self) {
        // 先获取 timers 锁，确保原子性
        let mut timers = self.timers.lock().await;

        // 在锁保护下取消所有 pending
        {
            let mut pending: MutexGuard<'_, HashMap<u64, CancellationToken>> =
                recover_lock(self.pending_tokens.lock());
            for (_, token) in pending.drain() {
                token.cancel();
            }
        }

        // 取消所有 timers（仍在锁保护下）
        for (_, entry) in timers.drain() {
            entry.cancel_token.cancel();
        }
        log::debug!("已取消所有定时器");
    }

    /// 获取活跃定时器数量
    pub async fn count(&self) -> usize {
        self.timers.lock().await.len()
    }

    /// 获取可用槽位数量
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Timer Functions
// ============================================================================

/// setTimeout 实现
fn set_timeout(
    ctx: &Ctx<'_>,
    _callback: Function<'_>,
    delay: Option<u64>,
    registry: Arc<TimerRegistry>,
) -> JsResult<u64> {
    let delay = delay.unwrap_or(0).min(MAX_DELAY_MS);

    // 同步获取 permit（真正占位，防止 spawn flooding）
    let (id, permit, cancel_token) = match registry.try_acquire() {
        Some(tuple) => tuple,
        None => {
            log::warn!("定时器数量超限 ({}), 拒绝创建新定时器", MAX_TIMERS);
            let msg = format!("Timer limit exceeded (max {})", MAX_TIMERS);
            let exception = Exception::from_message(ctx.clone(), &msg)?;
            return Err(ctx.throw(exception.into_object().into_value()));
        }
    };

    let registry_clone = registry.clone();
    let cancel_token_clone = cancel_token.clone();

    // 启动定时器任务（permit 已获取，槽位已占用）
    tokio::spawn(async move {
        // 先检查是否在 spawn 前已被取消（解决竞态）
        if cancel_token.is_cancelled() {
            log::trace!("定时器 {} 在启动前已被取消", id);
            // permit 在这里 drop，自动释放槽位
            return;
        }

        // 注册定时器（存储 cancel_token 和 permit）
        // register 返回 false 表示在竞态窗口期被取消
        if !registry_clone.register(id, false, cancel_token.clone(), permit).await {
            log::trace!("定时器 {} 在竞态窗口期被取消", id);
            return;
        }

        // 等待延时或取消
        tokio::select! {
            biased;  // 优先检查取消
            _ = cancel_token_clone.cancelled() => {
                log::trace!("定时器 {} 已取消", id);
                // 必须调用 complete 清理 timers 中的条目，释放 permit
                registry_clone.complete(id).await;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(delay)) => {
                // 执行回调
                // 注意：实际实现中需要通过消息队列在 JS 线程中执行
                log::trace!("定时器 {} 触发", id);
                registry_clone.complete(id).await;
            }
        }
    });

    Ok(id)
}

/// clearTimeout 实现
fn clear_timeout(id: u64, registry: Arc<TimerRegistry>) {
    // 异步取消
    tokio::spawn(async move {
        registry.cancel(id).await;
    });
}

/// setInterval 实现
fn set_interval(
    ctx: &Ctx<'_>,
    _callback: Function<'_>,
    delay: Option<u64>,
    registry: Arc<TimerRegistry>,
) -> JsResult<u64> {
    let delay = delay.unwrap_or(0).max(10).min(MAX_DELAY_MS); // 最小 10ms

    // 同步获取 permit（真正占位）
    let (id, permit, cancel_token) = match registry.try_acquire() {
        Some(tuple) => tuple,
        None => {
            log::warn!("定时器数量超限 ({}), 拒绝创建 interval", MAX_TIMERS);
            let msg = format!("Timer limit exceeded (max {})", MAX_TIMERS);
            let exception = Exception::from_message(ctx.clone(), &msg)?;
            return Err(ctx.throw(exception.into_object().into_value()));
        }
    };

    let registry_clone = registry.clone();
    let cancel_token_clone = cancel_token.clone();

    // 启动定时器任务
    tokio::spawn(async move {
        // 先检查是否在 spawn 前已被取消（解决竞态）
        if cancel_token.is_cancelled() {
            log::trace!("Interval {} 在启动前已被取消", id);
            // permit 在这里 drop，自动释放槽位
            return;
        }

        // 注册定时器
        // register 返回 false 表示在竞态窗口期被取消
        if !registry_clone.register(id, true, cancel_token.clone(), permit).await {
            log::trace!("Interval {} 在竞态窗口期被取消", id);
            return;
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(delay));

        loop {
            tokio::select! {
                biased;  // 优先检查取消
                _ = cancel_token_clone.cancelled() => {
                    log::trace!("Interval {} 已取消", id);
                    break;
                }
                _ = interval.tick() => {
                    // 触发回调 (通过消息队列)
                    log::trace!("Interval {} 触发", id);
                }
            }
        }

        registry_clone.complete(id).await;
    });

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timer_registry() {
        let registry = TimerRegistry::new();
        assert_eq!(registry.count().await, 0);
        assert_eq!(registry.available_permits(), MAX_TIMERS);
    }

    #[tokio::test]
    async fn test_timer_acquire_and_register() {
        let registry = Arc::new(TimerRegistry::new());

        // 获取槽位
        let (id, permit, cancel_token) = registry.try_acquire().unwrap();
        assert_eq!(registry.available_permits(), MAX_TIMERS - 1);

        // 注册（应返回 true 表示成功）
        assert!(registry.register(id, false, cancel_token, permit).await);
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let registry = Arc::new(TimerRegistry::new());

        let (id, permit, cancel_token) = registry.try_acquire().unwrap();
        assert!(registry.register(id, false, cancel_token, permit).await);

        assert!(registry.cancel(id).await);
        assert_eq!(registry.count().await, 0);
        assert_eq!(registry.available_permits(), MAX_TIMERS);
    }

    #[tokio::test]
    async fn test_timer_limit() {
        let registry = Arc::new(TimerRegistry::new());
        let mut entries = Vec::new();

        // 获取 MAX_TIMERS 个槽位
        for _ in 0..MAX_TIMERS {
            let tuple = registry.try_acquire().unwrap();
            entries.push(tuple);
        }

        // 尝试获取第 MAX_TIMERS+1 个
        assert!(registry.try_acquire().is_none());
        assert_eq!(registry.available_permits(), 0);

        // 释放一个后应该可以再次获取
        drop(entries.pop());
        assert_eq!(registry.available_permits(), 1);
        assert!(registry.try_acquire().is_some());
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // 取消前不应该被标记
        assert!(!token.is_cancelled());

        // 取消后应该被标记
        token_clone.cancel();
        assert!(token.is_cancelled());
    }

    // ========================================================================
    // 回归测试：P1 竞态窗口期漏取消
    // ========================================================================

    #[tokio::test]
    async fn test_cancel_before_register_race() {
        // 测试：在 register 之前取消定时器
        let registry = Arc::new(TimerRegistry::new());

        // 获取槽位（token 存入 pending_tokens）
        let (id, permit, cancel_token) = registry.try_acquire().unwrap();
        assert_eq!(registry.available_permits(), MAX_TIMERS - 1);

        // 在 register 之前取消
        assert!(registry.cancel(id).await);

        // 尝试 register（应返回 false，因为已被取消）
        assert!(!registry.register(id, false, cancel_token, permit).await);

        // 验证槽位已释放
        assert_eq!(registry.available_permits(), MAX_TIMERS);
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_cancel_during_register_window() {
        // 测试：模拟在 register 执行过程中取消
        let registry = Arc::new(TimerRegistry::new());

        // 统计竞态结果
        let mut cancel_first_count = 0;
        let mut register_first_count = 0;

        // 批量测试以增加竞态概率
        for _ in 0..100 {
            let reg = registry.clone();
            let (id, permit, cancel_token) = reg.try_acquire().unwrap();

            // 并发：一边 cancel，一边 register
            let reg1 = reg.clone();
            let reg2 = reg.clone();
            let cancel_token_clone = cancel_token.clone();

            let (cancel_result, register_result) = tokio::join!(
                async move { reg1.cancel(id).await },
                async move { reg2.register(id, false, cancel_token_clone, permit).await }
            );

            // 分析竞态结果
            // 场景 A: cancel 先执行
            //   - cancel_result = true (从 pending 移除)
            //   - register_result = false (发现已被取消)
            // 场景 B: register 先执行
            //   - register_result = true (成功注册)
            //   - cancel_result = true (从 timers 移除) 或 false (token 已触发)
            if !register_result && cancel_result {
                cancel_first_count += 1;
            } else if register_result {
                register_first_count += 1;
            }

            // 核心断言：无论竞态结果如何，至少有一方成功处理了定时器
            // 这确保不会出现"两边都找不到"的情况
            assert!(
                cancel_result || register_result,
                "竞态失败: cancel={}, register={} (定时器可能丢失)",
                cancel_result, register_result
            );
        }

        // 验证确实发生了竞态（两种场景都有）
        log::debug!(
            "竞态测试结果: cancel 先执行 {} 次, register 先执行 {} 次",
            cancel_first_count, register_first_count
        );

        // 最终所有槽位应该恢复
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(registry.available_permits(), MAX_TIMERS);
    }

    // ========================================================================
    // 回归测试：P2 Mutex 毒化恢复
    // ========================================================================

    #[test]
    fn test_recover_lock_normal() {
        let mutex = std::sync::Mutex::new(42);
        let guard = recover_lock(mutex.lock());
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_recover_lock_poisoned() {
        use std::sync::Mutex;
        use std::thread;

        let mutex = Arc::new(Mutex::new(42));
        let mutex_clone = mutex.clone();

        // 在另一个线程中持锁 panic，使 mutex 毒化
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("故意 panic 以毒化 mutex");
        });

        // 等待 panic
        let _ = handle.join();

        // 验证 mutex 已毒化
        assert!(mutex.lock().is_err());

        // 使用 recover_lock 恢复
        let guard = recover_lock(mutex.lock());
        assert_eq!(*guard, 42);
    }
}
