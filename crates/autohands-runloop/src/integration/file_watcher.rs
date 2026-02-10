//! File watcher trigger implementation.

#[cfg(test)]
#[path = "file_watcher_tests.rs"]
mod tests;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn};

use super::trigger_types::{FileWatcherConfig, TriggerError, TriggerEvent};

/// File watcher trigger that monitors file system changes.
pub struct FileWatcherTrigger {
    pub(crate) config: FileWatcherConfig,
    pub(crate) enabled: AtomicBool,
    pub(crate) event_sender: broadcast::Sender<TriggerEvent>,
    /// Watcher handle (Some when running).
    pub(crate) watcher: RwLock<Option<WatcherHandle>>,
}

/// Handle to the running watcher.
pub(crate) struct WatcherHandle {
    pub(crate) _watcher: RecommendedWatcher,
    pub(crate) shutdown_tx: mpsc::Sender<()>,
}

impl FileWatcherTrigger {
    /// Create a new file watcher trigger.
    pub fn new(config: FileWatcherConfig) -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            enabled: AtomicBool::new(config.enabled),
            config,
            event_sender: sender,
            watcher: RwLock::new(None),
        }
    }

    /// Subscribe to trigger events.
    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.event_sender.subscribe()
    }

    /// Check if a path matches the configured patterns.
    pub(crate) fn matches_pattern(&self, path: &PathBuf) -> bool {
        if self.config.patterns.is_empty() {
            return true;
        }
        let path_str = path.to_string_lossy();
        self.config.patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
        })
    }

    /// Handle a file event.
    pub fn handle_event(&self, paths: Vec<PathBuf>) -> Option<TriggerEvent> {
        if !self.enabled.load(Ordering::SeqCst) {
            return None;
        }

        let matched_paths: Vec<_> = paths
            .into_iter()
            .filter(|p| self.matches_pattern(p))
            .collect();

        if matched_paths.is_empty() {
            return None;
        }

        let event = TriggerEvent::new(
            &self.config.id,
            "file_watcher",
            &self.config.agent,
            &self.config.prompt,
        )
        .with_data(json!({
            "paths": matched_paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
        }));

        let _ = self.event_sender.send(event.clone());
        info!(
            "File watcher trigger fired: {} ({} files)",
            self.config.id,
            matched_paths.len()
        );

        Some(event)
    }

    /// Get the paths being watched.
    pub fn paths(&self) -> &[PathBuf] {
        &self.config.paths
    }

    /// Get the debounce delay.
    pub fn debounce_duration(&self) -> Duration {
        Duration::from_millis(self.config.debounce_ms)
    }

    /// Create the notify watcher and configure watched paths.
    pub(crate) fn create_watcher(
        &self,
        event_tx: mpsc::Sender<notify::Result<Event>>,
    ) -> Result<RecommendedWatcher, TriggerError> {
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = event_tx.blocking_send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| TriggerError::FileWatcher(format!("Failed to create watcher: {}", e)))?;

        for path in &self.config.paths {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    warn!("Failed to watch path {:?}: {}", path, e);
                } else {
                    info!("Watching path: {:?}", path);
                }
            } else {
                warn!("Watch path does not exist: {:?}", path);
            }
        }

        Ok(watcher)
    }
}
