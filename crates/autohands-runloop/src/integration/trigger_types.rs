//! Shared trigger types used by file_watcher and webhook modules.

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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
