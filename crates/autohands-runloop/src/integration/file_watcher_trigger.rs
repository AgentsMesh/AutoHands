//! Trigger trait implementation for FileWatcherTrigger and event processing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use super::file_watcher::{FileWatcherTrigger, WatcherHandle};
use super::trigger_types::{Trigger, TriggerError, TriggerEvent};

#[async_trait]
impl Trigger for FileWatcherTrigger {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn trigger_type(&self) -> &str {
        "file_watcher"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    async fn start(&self) -> Result<(), TriggerError> {
        {
            let handle = self.watcher.read().await;
            if handle.is_some() {
                warn!("File watcher {} is already running", self.config.id);
                return Ok(());
            }
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let (event_tx, event_rx) = mpsc::channel(100);

        let watcher = self.create_watcher(event_tx)?;

        {
            let mut handle = self.watcher.write().await;
            *handle = Some(WatcherHandle {
                _watcher: watcher,
                shutdown_tx,
            });
        }

        spawn_event_processor(
            self.config.id.clone(),
            self.config.patterns.clone(),
            self.config.agent.clone(),
            self.config.prompt.clone(),
            self.config.debounce_ms,
            self.event_sender.clone(),
            event_rx,
            shutdown_rx,
        );

        self.enabled.store(true, Ordering::SeqCst);
        info!("File watcher trigger started: {}", self.config.id);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerError> {
        self.enabled.store(false, Ordering::SeqCst);

        {
            let mut handle = self.watcher.write().await;
            if let Some(h) = handle.take() {
                let _ = h.shutdown_tx.send(()).await;
            }
        }

        info!("File watcher trigger stopped: {}", self.config.id);
        Ok(())
    }
}

/// Spawn event processing task for file watcher.
fn spawn_event_processor(
    trigger_id: String,
    patterns: Vec<String>,
    agent: String,
    prompt: String,
    debounce_ms: u64,
    event_sender: broadcast::Sender<TriggerEvent>,
    mut event_rx: mpsc::Receiver<notify::Result<notify::Event>>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut debounce_map: HashMap<PathBuf, Instant> = HashMap::new();
        let debounce_duration = Duration::from_millis(debounce_ms);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("File watcher {} shutting down", trigger_id);
                    break;
                }
                Some(result) = event_rx.recv() => {
                    if let Ok(event) = result {
                        let paths = filter_and_debounce(
                            event.paths,
                            &patterns,
                            &mut debounce_map,
                            debounce_duration,
                        );
                        if !paths.is_empty() {
                            let trigger_event = TriggerEvent::new(
                                &trigger_id, "file_watcher", &agent, &prompt,
                            ).with_data(json!({
                                "paths": paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
                                "event_kind": format!("{:?}", event.kind),
                            }));
                            if let Err(e) = event_sender.send(trigger_event) {
                                warn!("Failed to send trigger event: {}", e);
                            } else {
                                info!("File watcher {} triggered: {} files changed",
                                    trigger_id, paths.len());
                            }
                        }
                    } else if let Err(e) = result {
                        error!("File watcher {} error: {}", trigger_id, e);
                    }
                }
            }
        }
    });
}

/// Filter paths by pattern and debounce.
fn filter_and_debounce(
    paths: Vec<PathBuf>,
    patterns: &[String],
    debounce_map: &mut HashMap<PathBuf, Instant>,
    debounce_duration: Duration,
) -> Vec<PathBuf> {
    let now = Instant::now();
    let mut result = Vec::new();

    for path in paths {
        if let Some(last_time) = debounce_map.get(&path) {
            if now.duration_since(*last_time) < debounce_duration {
                debug!("Debouncing event for {:?}", path);
                continue;
            }
        }

        let path_str = path.to_string_lossy();
        let matches = if patterns.is_empty() {
            true
        } else {
            patterns.iter().any(|p| {
                glob::Pattern::new(p)
                    .map(|pat| pat.matches(&path_str))
                    .unwrap_or(false)
            })
        };

        if matches {
            debounce_map.insert(path.clone(), now);
            result.push(path);
        }
    }

    result
}
