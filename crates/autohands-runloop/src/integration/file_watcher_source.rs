//! File watcher RunLoop Source1 adapter.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::debug;

use crate::error::RunLoopResult;
use crate::mode::RunLoopMode;
use crate::source::{PortMessage, Source1, Source1Receiver};
use crate::task::{Task, TaskPriority, TaskSource};

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
