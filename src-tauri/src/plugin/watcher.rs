// 插件热重载监听器
// Phase 2.3.8-2.3.9: 集成 notify 实现热重载
//
// 监听插件文件变化，触发 unload → load 热重载

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

// ============================================================================
// 热重载事件
// ============================================================================

/// 热重载事件类型
#[derive(Debug, Clone)]
pub enum HotReloadEvent {
    /// 插件文件已修改
    Modified { plugin_id: String, path: PathBuf },
    /// 插件目录已创建 (新插件)
    Created { path: PathBuf },
    /// 插件目录已删除
    Removed { plugin_id: String, path: PathBuf },
    /// 监听错误
    Error { message: String },
}

// ============================================================================
// 插件文件监听器
// ============================================================================

/// 插件文件监听器
pub struct PluginWatcher {
    /// 监听器实例 (需要保持存活)
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    /// 事件接收通道
    rx: mpsc::Receiver<HotReloadEvent>,
}

impl PluginWatcher {
    /// 创建新的监听器
    pub fn new(plugins_dir: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel(100);
        let plugins_dir_clone = plugins_dir.to_path_buf();

        // 创建监听器（使用非阻塞发送，防止阻塞 notify 线程）
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        if let Some(hot_event) = Self::process_event_static(&event, &plugins_dir_clone) {
                            // 使用 try_send 而非 blocking_send，避免阻塞 notify 线程
                            // 如果通道满，丢弃事件并记录警告
                            if let Err(e) = tx.try_send(hot_event) {
                                match e {
                                    mpsc::error::TrySendError::Full(_) => {
                                        log::warn!("插件监听器事件通道已满，丢弃事件");
                                    }
                                    mpsc::error::TrySendError::Closed(_) => {
                                        log::error!("插件监听器事件通道已关闭");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // 错误事件也使用非阻塞发送
                        let _ = tx.try_send(HotReloadEvent::Error {
                            message: e.to_string(),
                        });
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        // 开始监听
        let mut watcher = watcher;
        watcher.watch(plugins_dir, RecursiveMode::Recursive)?;

        log::info!("已启动插件目录监听: {:?}", plugins_dir);

        Ok(Self { watcher, rx })
    }

    /// 处理文件系统事件 (静态方法)
    fn process_event_static(event: &Event, plugins_dir: &Path) -> Option<HotReloadEvent> {
        use notify::EventKind;

        // 获取事件涉及的路径
        let path = event.paths.first()?;

        // 计算相对路径
        let relative = path.strip_prefix(plugins_dir).ok()?;

        // 获取插件 ID (第一级目录名)
        let plugin_id = relative.iter().next()?.to_string_lossy().to_string();

        match event.kind {
            EventKind::Create(_) => {
                // 新文件/目录创建
                if path.is_dir() && relative.components().count() == 1 {
                    Some(HotReloadEvent::Created {
                        path: path.clone(),
                    })
                } else {
                    None
                }
            }
            EventKind::Modify(_) => {
                // 只关注 .js 和 .json 文件
                if let Some(ext) = path.extension() {
                    if ext == "js" || ext == "json" {
                        return Some(HotReloadEvent::Modified {
                            plugin_id,
                            path: path.clone(),
                        });
                    }
                }
                None
            }
            EventKind::Remove(_) => {
                if relative.components().count() == 1 {
                    Some(HotReloadEvent::Removed {
                        plugin_id,
                        path: path.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 接收下一个事件
    pub async fn recv(&mut self) -> Option<HotReloadEvent> {
        self.rx.recv().await
    }

    /// 尝试接收事件 (非阻塞)
    pub fn try_recv(&mut self) -> Option<HotReloadEvent> {
        self.rx.try_recv().ok()
    }
}

// ============================================================================
// 热重载管理器
// ============================================================================

/// 热重载管理器
pub struct HotReloadManager {
    plugins_dir: PathBuf,
    #[allow(dead_code)]
    debounce_ms: u64,
}

impl HotReloadManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            debounce_ms: 500,
        }
    }

    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    pub fn start(&self) -> Result<PluginWatcher, notify::Error> {
        PluginWatcher::new(&self.plugins_dir)
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_event_debug() {
        let event = HotReloadEvent::Modified {
            plugin_id: "test".to_string(),
            path: PathBuf::from("/plugins/test/plugin.js"),
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Modified"));
    }
}
