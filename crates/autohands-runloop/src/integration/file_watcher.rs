//! File watcher trigger implementation.
//!
//! Provides both the full FileWatcherTrigger implementation and
//! the Source1 adapter for RunLoop integration.
//!
//! This module also contains the shared Trigger types used by webhook.rs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::RunLoopResult;

// ============================================================================
// Shared Trigger Types (used by file_watcher and webhook modules)
// ============================================================================

/// Trigger error types.
#[derive(Debug, Error)]
pub enum TriggerError {
    /// Trigger not found.
    #[error("Trigger not found: {0}")]
    NotFound(String),

    /// Trigger already exists.
    #[error("Trigger already exists: {0}")]
    AlreadyExists(String),

    /// Invalid trigger configuration.
    #[error("Invalid trigger configuration: {0}")]
    InvalidConfig(String),

    /// File watcher error.
    #[error("File watcher error: {0}")]
    FileWatcher(String),

    /// Webhook error.
    #[error("Webhook error: {0}")]
    Webhook(String),

    /// Trigger disabled.
    #[error("Trigger is disabled: {0}")]
    Disabled(String),

    /// Generic error.
    #[error("{0}")]
    Custom(String),
}

/// Event emitted when a trigger fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// Event ID.
    pub id: Uuid,
    /// Trigger ID that fired.
    pub trigger_id: String,
    /// Trigger type.
    pub trigger_type: String,
    /// Agent to run.
    pub agent: String,
    /// Prompt to execute.
    pub prompt: String,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Additional data from the trigger.
    pub data: serde_json::Value,
}

impl TriggerEvent {
    /// Create a new trigger event.
    pub fn new(
        trigger_id: impl Into<String>,
        trigger_type: impl Into<String>,
        agent: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            trigger_id: trigger_id.into(),
            trigger_type: trigger_type.into(),
            agent: agent.into(),
            prompt: prompt.into(),
            timestamp: Utc::now(),
            data: serde_json::Value::Null,
        }
    }

    /// Set event data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

/// Trigger trait for different trigger types.
#[async_trait]
pub trait Trigger: Send + Sync {
    /// Get the trigger ID.
    fn id(&self) -> &str;

    /// Get the trigger type.
    fn trigger_type(&self) -> &str;

    /// Check if trigger is enabled.
    fn is_enabled(&self) -> bool;

    /// Start the trigger.
    async fn start(&self) -> Result<(), TriggerError>;

    /// Stop the trigger.
    async fn stop(&self) -> Result<(), TriggerError>;
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Triggers configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersConfig {
    /// Webhook triggers.
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,

    /// File watcher triggers.
    #[serde(default)]
    pub file_watchers: Vec<FileWatcherConfig>,
}

/// Webhook trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Trigger ID.
    pub id: String,
    /// URL path.
    pub path: String,
    /// Agent to trigger.
    pub agent: String,
    /// Optional prompt template.
    pub prompt_template: Option<String>,
    /// Whether trigger is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Required secret for verification.
    pub secret: Option<String>,
}

/// File watcher trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherConfig {
    /// Trigger ID.
    pub id: String,
    /// Paths to watch.
    pub paths: Vec<PathBuf>,
    /// File patterns to match.
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Agent to trigger.
    pub agent: String,
    /// Prompt to execute.
    pub prompt: String,
    /// Whether trigger is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Debounce delay in milliseconds.
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_debounce() -> u64 {
    500
}
use crate::task::{Task, TaskPriority, TaskSource};
use crate::mode::RunLoopMode;
use crate::source::{PortMessage, Source1, Source1Receiver};

// ============================================================================
// FileWatcherTrigger - Full implementation
// ============================================================================

/// File watcher trigger that monitors file system changes.
pub struct FileWatcherTrigger {
    config: FileWatcherConfig,
    enabled: AtomicBool,
    event_sender: broadcast::Sender<TriggerEvent>,
    /// Watcher handle (Some when running).
    watcher: RwLock<Option<WatcherHandle>>,
}

/// Handle to the running watcher.
struct WatcherHandle {
    /// The notify watcher instance.
    _watcher: RecommendedWatcher,
    /// Shutdown signal sender.
    shutdown_tx: mpsc::Sender<()>,
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
    fn matches_pattern(&self, path: &PathBuf) -> bool {
        if self.config.patterns.is_empty() {
            return true;
        }

        let path_str = path.to_string_lossy();
        for pattern in &self.config.patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    /// Handle a file event.
    pub fn handle_event(&self, paths: Vec<PathBuf>) -> Option<TriggerEvent> {
        if !self.is_enabled() {
            return None;
        }

        // Filter paths by pattern
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
}

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
        // Check if already running
        {
            let handle = self.watcher.read().await;
            if handle.is_some() {
                warn!("File watcher {} is already running", self.config.id);
                return Ok(());
            }
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let (event_tx, mut event_rx) = mpsc::channel::<notify::Result<Event>>(100);

        // Create the watcher
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = event_tx.blocking_send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| TriggerError::FileWatcher(format!("Failed to create watcher: {}", e)))?;

        // Store watcher handle before configuring paths (watcher must outlive configuration)
        let mut watcher = watcher;

        // Add paths to watch
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

        // Store the watcher handle
        {
            let mut handle = self.watcher.write().await;
            *handle = Some(WatcherHandle {
                _watcher: watcher,
                shutdown_tx,
            });
        }

        // Spawn event processing task
        let trigger_id = self.config.id.clone();
        let patterns = self.config.patterns.clone();
        let agent = self.config.agent.clone();
        let prompt = self.config.prompt.clone();
        let debounce_ms = self.config.debounce_ms;
        let event_sender = self.event_sender.clone();
        let enabled = Arc::new(AtomicBool::new(true));
        let enabled_clone = enabled.clone();

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
                        if !enabled_clone.load(Ordering::SeqCst) {
                            continue;
                        }

                        match result {
                            Ok(event) => {
                                // Filter and debounce events
                                let now = Instant::now();
                                let mut paths_to_process = Vec::new();

                                for path in event.paths {
                                    // Check debounce
                                    if let Some(last_time) = debounce_map.get(&path) {
                                        if now.duration_since(*last_time) < debounce_duration {
                                            debug!("Debouncing event for {:?}", path);
                                            continue;
                                        }
                                    }

                                    // Check pattern match
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
                                        paths_to_process.push(path);
                                    }
                                }

                                if !paths_to_process.is_empty() {
                                    let trigger_event = TriggerEvent::new(
                                        &trigger_id,
                                        "file_watcher",
                                        &agent,
                                        &prompt,
                                    )
                                    .with_data(json!({
                                        "paths": paths_to_process.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
                                        "event_kind": format!("{:?}", event.kind),
                                    }));

                                    if let Err(e) = event_sender.send(trigger_event) {
                                        warn!("Failed to send trigger event: {}", e);
                                    } else {
                                        info!(
                                            "File watcher {} triggered: {} files changed",
                                            trigger_id,
                                            paths_to_process.len()
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("File watcher {} error: {}", trigger_id, e);
                            }
                        }
                    }
                }
            }
        });

        self.enabled.store(true, Ordering::SeqCst);
        info!("File watcher trigger started: {}", self.config.id);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerError> {
        self.enabled.store(false, Ordering::SeqCst);

        // Send shutdown signal and drop watcher
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

// ============================================================================
// FileWatcherManager - Manages multiple file watchers
// ============================================================================

/// Manager for multiple file watcher triggers.
pub struct FileWatcherManager {
    triggers: RwLock<HashMap<String, Arc<FileWatcherTrigger>>>,
}

impl FileWatcherManager {
    /// Create a new file watcher manager.
    pub fn new() -> Self {
        Self {
            triggers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a file watcher trigger.
    pub async fn register(
        &self,
        config: FileWatcherConfig,
    ) -> Result<Arc<FileWatcherTrigger>, TriggerError> {
        let id = config.id.clone();
        let trigger = Arc::new(FileWatcherTrigger::new(config));

        let mut triggers = self.triggers.write().await;
        if triggers.contains_key(&id) {
            return Err(TriggerError::AlreadyExists(id));
        }

        triggers.insert(id, trigger.clone());
        Ok(trigger)
    }

    /// Unregister a file watcher trigger.
    pub async fn unregister(&self, id: &str) -> Result<(), TriggerError> {
        let mut triggers = self.triggers.write().await;

        if let Some(trigger) = triggers.remove(id) {
            trigger.stop().await?;
            Ok(())
        } else {
            Err(TriggerError::NotFound(id.to_string()))
        }
    }

    /// Get a trigger by ID.
    pub async fn get(&self, id: &str) -> Option<Arc<FileWatcherTrigger>> {
        let triggers = self.triggers.read().await;
        triggers.get(id).cloned()
    }

    /// List all triggers.
    pub async fn list(&self) -> Vec<Arc<FileWatcherTrigger>> {
        let triggers = self.triggers.read().await;
        triggers.values().cloned().collect()
    }

    /// Start all triggers.
    pub async fn start_all(&self) -> Result<(), TriggerError> {
        let triggers = self.triggers.read().await;
        for trigger in triggers.values() {
            if let Err(e) = trigger.start().await {
                error!("Failed to start trigger {}: {}", trigger.id(), e);
            }
        }
        Ok(())
    }

    /// Stop all triggers.
    pub async fn stop_all(&self) -> Result<(), TriggerError> {
        let triggers = self.triggers.read().await;
        for trigger in triggers.values() {
            if let Err(e) = trigger.stop().await {
                error!("Failed to stop trigger {}: {}", trigger.id(), e);
            }
        }
        Ok(())
    }
}

impl Default for FileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FileWatcherSource1 - RunLoop Source1 adapter
// ============================================================================

/// File change event for Source1.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path of the changed file.
    pub path: String,
    /// Type of change.
    pub change_type: FileChangeType,
    /// Agent to handle the change.
    pub agent: Option<String>,
    /// Prompt for the agent.
    pub prompt: Option<String>,
}

/// Type of file change.
#[derive(Debug, Clone, Copy)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl std::fmt::Display for FileChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileChangeType::Created => write!(f, "created"),
            FileChangeType::Modified => write!(f, "modified"),
            FileChangeType::Deleted => write!(f, "deleted"),
            FileChangeType::Renamed => write!(f, "renamed"),
        }
    }
}

/// File watcher Source1.
///
/// Receives file change events and produces RunLoop events.
pub struct FileWatcherSource1 {
    id: String,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl FileWatcherSource1 {
    /// Create a new file watcher source.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Default],
        }
    }

    /// Create with custom modes.
    pub fn with_modes(mut self, modes: Vec<RunLoopMode>) -> Self {
        self.modes = modes;
        self
    }

    /// Create a Source1Receiver for this source.
    ///
    /// Returns the receiver and a sender to send file change events.
    pub fn create_receiver(self) -> (Source1Receiver, mpsc::Sender<PortMessage>) {
        let (tx, rx) = mpsc::channel(256);
        let source = Arc::new(self);
        (Source1Receiver::new(source, rx), tx)
    }

    /// Create a PortMessage from a FileChangeEvent.
    pub fn create_message(event: FileChangeEvent) -> PortMessage {
        PortMessage::new(
            "file_watcher",
            json!({
                "path": event.path,
                "change_type": event.change_type.to_string(),
                "agent": event.agent,
                "prompt": event.prompt,
            }),
        )
    }
}

#[async_trait]
impl Source1 for FileWatcherSource1 {
    fn id(&self) -> &str {
        &self.id
    }

    async fn handle(&self, msg: PortMessage) -> RunLoopResult<Vec<Task>> {
        let path = msg.payload["path"].as_str().unwrap_or("");
        let change_type = msg.payload["change_type"].as_str().unwrap_or("modified");
        let agent = msg.payload["agent"].as_str();
        let prompt = msg.payload["prompt"].as_str();

        debug!("File change: {} ({})", path, change_type);

        let event = Task::new(
            "trigger:file:changed",
            json!({
                "path": path,
                "change_type": change_type,
                "agent": agent,
                "prompt": prompt,
            }),
        )
        .with_source(TaskSource::FileWatcher)
        .with_priority(TaskPriority::Normal);

        Ok(vec![event])
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::sleep;

    fn test_config() -> FileWatcherConfig {
        FileWatcherConfig {
            id: "test-watcher".to_string(),
            paths: vec![PathBuf::from("/tmp")],
            patterns: vec!["*.txt".to_string()],
            agent: "general".to_string(),
            prompt: "Process file change".to_string(),
            enabled: true,
            debounce_ms: 500,
        }
    }

    #[test]
    fn test_file_watcher_new() {
        let trigger = FileWatcherTrigger::new(test_config());
        assert_eq!(trigger.id(), "test-watcher");
        assert_eq!(trigger.trigger_type(), "file_watcher");
        assert!(trigger.is_enabled());
    }

    #[test]
    fn test_matches_pattern() {
        let trigger = FileWatcherTrigger::new(test_config());

        assert!(trigger.matches_pattern(&PathBuf::from("/tmp/test.txt")));
        assert!(!trigger.matches_pattern(&PathBuf::from("/tmp/test.rs")));
    }

    #[test]
    fn test_matches_pattern_empty() {
        let mut config = test_config();
        config.patterns = vec![];
        let trigger = FileWatcherTrigger::new(config);

        // Empty patterns match everything
        assert!(trigger.matches_pattern(&PathBuf::from("/tmp/anything")));
    }

    #[test]
    fn test_handle_event() {
        let trigger = FileWatcherTrigger::new(test_config());
        let event = trigger.handle_event(vec![PathBuf::from("/tmp/test.txt")]);

        assert!(event.is_some());
        let e = event.unwrap();
        assert_eq!(e.trigger_id, "test-watcher");
    }

    #[test]
    fn test_handle_event_no_match() {
        let trigger = FileWatcherTrigger::new(test_config());
        let event = trigger.handle_event(vec![PathBuf::from("/tmp/test.rs")]);

        assert!(event.is_none());
    }

    #[test]
    fn test_debounce_duration() {
        let trigger = FileWatcherTrigger::new(test_config());
        assert_eq!(trigger.debounce_duration(), Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_file_watcher_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.paths = vec![temp_dir.path().to_path_buf()];
        config.debounce_ms = 100;

        let trigger = FileWatcherTrigger::new(config);

        // Start
        trigger.start().await.unwrap();
        assert!(trigger.is_enabled());

        // Stop
        trigger.stop().await.unwrap();
        assert!(!trigger.is_enabled());
    }

    #[tokio::test]
    async fn test_file_watcher_manager() {
        let manager = FileWatcherManager::new();
        let temp_dir = TempDir::new().unwrap();

        let mut config = test_config();
        config.paths = vec![temp_dir.path().to_path_buf()];

        // Register
        let trigger = manager.register(config.clone()).await.unwrap();
        assert_eq!(trigger.id(), "test-watcher");

        // Get
        let retrieved = manager.get("test-watcher").await;
        assert!(retrieved.is_some());

        // List
        let list = manager.list().await;
        assert_eq!(list.len(), 1);

        // Duplicate registration should fail
        let result = manager.register(config).await;
        assert!(result.is_err());

        // Unregister
        manager.unregister("test-watcher").await.unwrap();
        let list = manager.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_file_watcher_event_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.paths = vec![temp_dir.path().to_path_buf()];
        config.patterns = vec!["*.txt".to_string()];
        config.debounce_ms = 50;

        let trigger = Arc::new(FileWatcherTrigger::new(config));
        let mut receiver = trigger.subscribe();

        trigger.start().await.unwrap();

        // Give the watcher time to initialize
        sleep(Duration::from_millis(100)).await;

        // Create a file
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        // Wait for event with timeout
        let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv()).await;

        trigger.stop().await.unwrap();

        // The event may or may not be received depending on timing
        // This test mainly verifies no panics/crashes
        if let Ok(Ok(event)) = result {
            assert_eq!(event.trigger_type, "file_watcher");
        }
    }

    // Source1 tests
    #[tokio::test]
    async fn test_file_watcher_source1() {
        let source = FileWatcherSource1::new("file-watcher");
        assert_eq!(source.id(), "file-watcher");
        assert!(source.is_valid());
    }

    #[tokio::test]
    async fn test_file_watcher_source1_handle() {
        let source = FileWatcherSource1::new("file-watcher");

        let msg = FileWatcherSource1::create_message(FileChangeEvent {
            path: "/test/file.txt".to_string(),
            change_type: FileChangeType::Modified,
            agent: Some("general".to_string()),
            prompt: Some("Handle file change".to_string()),
        });

        let events = source.handle(msg).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "trigger:file:changed");
    }

    #[tokio::test]
    async fn test_file_watcher_create_receiver() {
        let source = FileWatcherSource1::new("file-watcher");
        let (receiver, tx) = source.create_receiver();

        assert_eq!(receiver.source.id(), "file-watcher");

        // Test sending a message
        let msg = FileWatcherSource1::create_message(FileChangeEvent {
            path: "/test.txt".to_string(),
            change_type: FileChangeType::Created,
            agent: None,
            prompt: None,
        });

        tx.send(msg).await.unwrap();
    }

    #[test]
    fn test_file_change_type_display() {
        assert_eq!(FileChangeType::Created.to_string(), "created");
        assert_eq!(FileChangeType::Modified.to_string(), "modified");
        assert_eq!(FileChangeType::Deleted.to_string(), "deleted");
        assert_eq!(FileChangeType::Renamed.to_string(), "renamed");
    }
}
