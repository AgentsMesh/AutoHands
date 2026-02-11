//! File watcher event injector.
//!
//! Converts file change events into tasks via `TaskSubmitter`.
//! Decoupled from RunLoop internals.

use std::sync::Arc;

use serde_json::json;

use autohands_protocols::extension::TaskSubmitter;

/// File change event.
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

/// File watcher event injector.
///
/// Converts file change events into tasks via `TaskSubmitter`.
/// Decoupled from RunLoop internals.
pub struct FileWatcherInjector {
    task_submitter: Arc<dyn TaskSubmitter>,
}

impl FileWatcherInjector {
    /// Create a new file watcher injector.
    pub fn new(task_submitter: Arc<dyn TaskSubmitter>) -> Self {
        Self { task_submitter }
    }

    /// Inject a file change event as a task.
    pub async fn inject(&self, event: FileChangeEvent) -> Result<(), autohands_protocols::error::ExtensionError> {
        self.task_submitter
            .submit_task(
                "trigger:file:changed",
                json!({
                    "path": event.path,
                    "change_type": event.change_type.to_string(),
                    "agent": event.agent,
                    "prompt": event.prompt,
                }),
                None,
            )
            .await
    }
}
