// 应用状态管理模块
// Phase 2+ 预留，当前未使用
#![allow(dead_code)]

use std::sync::Mutex;

/// 应用全局状态
pub struct AppState {
    /// 是否已完成首次设置
    pub setup_completed: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            setup_completed: Mutex::new(false),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
