// Phase 6.1.2: 滑动窗口成功率统计
// 基于环形缓冲区实现最近 N 次调用的统计

use std::collections::VecDeque;
use std::time::Instant;

/// 默认滑动窗口大小
pub const DEFAULT_WINDOW_SIZE: usize = 100;

/// 单次调用结果
#[derive(Debug, Clone)]
pub struct CallResult {
    /// 是否成功
    pub success: bool,
    /// 延迟 (毫秒)
    pub latency_ms: f64,
    /// 调用时间
    pub timestamp: Instant,
}

/// 滑动窗口统计
///
/// 使用环形缓冲区存储最近 N 次调用结果，
/// 提供成功率、平均延迟、P99 延迟等统计。
///
/// 所有统计方法只需 `&self`，支持并发读取。
#[derive(Debug)]
pub struct SlidingWindow {
    /// 调用结果队列
    results: VecDeque<CallResult>,
    /// 窗口大小（最小为 1）
    window_size: usize,
}

impl SlidingWindow {
    /// 创建新的滑动窗口
    ///
    /// # 参数
    /// - `window_size`: 窗口大小，最小为 1（传入 0 会自动调整为 1）
    pub fn new(window_size: usize) -> Self {
        // P3 修复：window_size=0 边界防护
        let effective_size = window_size.max(1);
        Self {
            results: VecDeque::with_capacity(effective_size),
            window_size: effective_size,
        }
    }

    /// 使用默认窗口大小创建
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_WINDOW_SIZE)
    }

    /// 记录成功调用
    pub fn record_success(&mut self, latency_ms: f64) {
        self.push_result(CallResult {
            success: true,
            latency_ms,
            timestamp: Instant::now(),
        });
    }

    /// 记录失败调用
    pub fn record_failure(&mut self, latency_ms: f64) {
        self.push_result(CallResult {
            success: false,
            latency_ms,
            timestamp: Instant::now(),
        });
    }

    /// 推入新结果
    fn push_result(&mut self, result: CallResult) {
        // 窗口满时移除最旧的记录
        if self.results.len() >= self.window_size {
            self.results.pop_front();
        }
        self.results.push_back(result);
    }

    /// 获取当前窗口中的调用次数
    pub fn count(&self) -> usize {
        self.results.len()
    }

    /// 获取成功率 (0.0-1.0)
    ///
    /// 如果窗口为空，返回 1.0（假设健康）
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 1.0;
        }
        let success_count = self.results.iter().filter(|r| r.success).count();
        success_count as f64 / self.results.len() as f64
    }

    /// 获取平均延迟 (毫秒)
    ///
    /// 只统计成功调用的有效延迟值（有限且非负）。
    ///
    /// 优化：
    /// - 过滤 NaN/inf，避免 IPC 序列化失败
    /// - 过滤负值，确保延迟语义正确
    /// - 返回值兜底检查，极端大值溢出时返回 0.0
    pub fn avg_latency_ms(&self) -> f64 {
        let valid_latencies: Vec<f64> = self.results.iter()
            .filter(|r| r.success && r.latency_ms.is_finite() && r.latency_ms >= 0.0)
            .map(|r| r.latency_ms)
            .collect();

        if valid_latencies.is_empty() {
            return 0.0;
        }

        let sum: f64 = valid_latencies.iter().sum();
        let avg = sum / valid_latencies.len() as f64;

        // 兜底检查：极端大值溢出时返回 0.0
        if avg.is_finite() { avg } else { 0.0 }
    }

    /// 获取 P99 延迟 (毫秒)
    ///
    /// 返回成功调用中第 99 百分位的延迟。
    ///
    /// 优化：
    /// - 使用 `f64::total_cmp` 确保排序稳定性
    /// - 过滤 NaN/inf/负值，确保延迟语义正确
    pub fn p99_latency_ms(&self) -> f64 {
        // 只考虑成功调用的有效延迟值（有限且非负）
        let mut valid_latencies: Vec<f64> = self.results.iter()
            .filter(|r| r.success && r.latency_ms.is_finite() && r.latency_ms >= 0.0)
            .map(|r| r.latency_ms)
            .collect();

        if valid_latencies.is_empty() {
            return 0.0;
        }

        // 使用 f64::total_cmp 确保全序，避免 NaN 破坏排序
        valid_latencies.sort_by(|a, b| a.total_cmp(b));

        // 计算 P99 索引
        let p99_index = ((valid_latencies.len() as f64 * 0.99).ceil() as usize).saturating_sub(1);
        valid_latencies.get(p99_index).copied().unwrap_or(0.0)
    }

    /// 获取成功调用次数
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.success).count()
    }

    /// 获取失败调用次数
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }

    /// 清空窗口
    pub fn clear(&mut self) {
        self.results.clear();
    }

    /// 获取统计快照
    ///
    /// 只需 `&self`，支持并发读取。
    pub fn snapshot(&self) -> WindowStats {
        WindowStats {
            window_size: self.window_size,
            call_count: self.count(),
            success_count: self.success_count(),
            failure_count: self.failure_count(),
            success_rate: self.success_rate(),
            avg_latency_ms: self.avg_latency_ms(),
            p99_latency_ms: self.p99_latency_ms(),
        }
    }

    /// 获取窗口大小
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self::with_default_size()
    }
}

/// 窗口统计快照
#[derive(Debug, Clone)]
pub struct WindowStats {
    /// 窗口大小
    pub window_size: usize,
    /// 当前调用次数
    pub call_count: usize,
    /// 成功次数
    pub success_count: usize,
    /// 失败次数
    pub failure_count: usize,
    /// 成功率
    pub success_rate: f64,
    /// 平均延迟
    pub avg_latency_ms: f64,
    /// P99 延迟
    pub p99_latency_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_window() {
        let window = SlidingWindow::new(10);
        assert_eq!(window.count(), 0);
        assert_eq!(window.success_rate(), 1.0);
        assert_eq!(window.avg_latency_ms(), 0.0);
        assert_eq!(window.p99_latency_ms(), 0.0);
    }

    #[test]
    fn test_success_rate() {
        let mut window = SlidingWindow::new(10);

        // 记录 8 次成功，2 次失败
        for _ in 0..8 {
            window.record_success(100.0);
        }
        for _ in 0..2 {
            window.record_failure(50.0);
        }

        assert_eq!(window.count(), 10);
        assert!((window.success_rate() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_sliding_window_overflow() {
        let mut window = SlidingWindow::new(5);

        // 记录 5 次失败
        for _ in 0..5 {
            window.record_failure(100.0);
        }
        assert_eq!(window.success_rate(), 0.0);

        // 再记录 5 次成功，旧的失败应该被推出
        for _ in 0..5 {
            window.record_success(50.0);
        }
        assert_eq!(window.count(), 5);
        assert_eq!(window.success_rate(), 1.0);
    }

    #[test]
    fn test_avg_latency() {
        let mut window = SlidingWindow::new(10);

        // 记录不同延迟
        window.record_success(100.0);
        window.record_success(200.0);
        window.record_success(300.0);
        window.record_failure(1000.0); // 失败调用不计入平均

        assert!((window.avg_latency_ms() - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_avg_latency_filters_invalid_values() {
        let mut window = SlidingWindow::new(10);

        // 记录正常值和异常值（NaN/inf/负值）
        window.record_success(100.0);
        window.record_success(f64::NAN);
        window.record_success(f64::INFINITY);
        window.record_success(200.0);
        window.record_success(f64::NEG_INFINITY);
        window.record_success(-50.0); // 负值也应被过滤
        window.record_success(300.0);

        // 平均值应该只考虑有效值 [100, 200, 300] = 200
        let avg = window.avg_latency_ms();
        assert!((avg - 200.0).abs() < 0.001);
        assert!(avg.is_finite());
    }

    #[test]
    fn test_avg_latency_overflow_fallback() {
        let mut window = SlidingWindow::new(10);

        // 记录极端大值
        window.record_success(f64::MAX);
        window.record_success(f64::MAX);

        // 溢出时应该返回 0.0（兜底）
        let avg = window.avg_latency_ms();
        assert!(avg.is_finite() || avg == 0.0);
    }

    #[test]
    fn test_p99_latency() {
        let mut window = SlidingWindow::new(100);

        // 记录 100 次调用，延迟从 1 到 100
        for i in 1..=100 {
            window.record_success(i as f64);
        }

        // P99 应该是第 99 个值（99 毫秒）
        let p99 = window.p99_latency_ms();
        assert!(p99 >= 99.0 && p99 <= 100.0);
    }

    #[test]
    fn test_p99_latency_filters_invalid_values() {
        let mut window = SlidingWindow::new(10);

        // 记录正常值和异常值（NaN/inf/负值）
        window.record_success(100.0);
        window.record_success(f64::NAN);
        window.record_success(f64::INFINITY);
        window.record_success(200.0);
        window.record_success(f64::NEG_INFINITY);
        window.record_success(-50.0); // 负值也应被过滤
        window.record_success(300.0);

        // P99 应该只考虑有效值 [100, 200, 300]
        let p99 = window.p99_latency_ms();
        assert!(p99 >= 200.0 && p99 <= 300.0);
        assert!(p99.is_finite());
    }

    #[test]
    fn test_snapshot() {
        let mut window = SlidingWindow::new(10);

        window.record_success(100.0);
        window.record_success(200.0);
        window.record_failure(50.0);

        let stats = window.snapshot();
        assert_eq!(stats.call_count, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
    }

    #[test]
    fn test_window_size_zero_protection() {
        // P3 修复：window_size=0 应该被自动调整为 1
        let window = SlidingWindow::new(0);
        assert_eq!(window.window_size(), 1);

        let mut window = SlidingWindow::new(0);
        window.record_success(100.0);
        window.record_success(200.0);
        // 窗口大小为 1，只保留最新的一条
        assert_eq!(window.count(), 1);
        assert!((window.avg_latency_ms() - 200.0).abs() < 0.001);
    }
}
