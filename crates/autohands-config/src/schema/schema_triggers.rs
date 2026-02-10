//! Trigger and daemon configuration types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::default_true;

/// Daemon configuration for 24/7 operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Whether daemon mode is enabled.
    #[serde(default = "default_daemon_enabled")]
    pub enabled: bool,

    /// Path to PID file.
    #[serde(default)]
    pub pid_file: Option<PathBuf>,

    /// Whether to automatically restart on crash.
    #[serde(default = "default_true")]
    pub auto_restart: bool,

    /// Maximum number of restarts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,

    /// Graceful shutdown timeout in seconds.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

fn default_daemon_enabled() -> bool {
    false
}

fn default_max_restarts() -> u32 {
    10
}

fn default_shutdown_timeout() -> u64 {
    30
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: default_daemon_enabled(),
            pid_file: None,
            auto_restart: default_true(),
            max_restarts: default_max_restarts(),
            shutdown_timeout_secs: default_shutdown_timeout(),
        }
    }
}

/// Triggers configuration for event-driven execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersConfig {
    /// Webhook triggers.
    #[serde(default)]
    pub webhooks: Vec<WebhookTriggerConfig>,

    /// File watcher triggers.
    #[serde(default)]
    pub file_watchers: Vec<FileWatcherTriggerConfig>,
}

/// Webhook trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTriggerConfig {
    /// Trigger ID.
    pub id: String,
    /// URL path.
    pub path: String,
    /// Agent to trigger.
    pub agent: String,
}

/// File watcher trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherTriggerConfig {
    /// Trigger ID.
    pub id: String,
    /// Paths to watch.
    pub paths: Vec<PathBuf>,
    /// Agent to trigger.
    pub agent: String,
    /// Prompt to execute.
    pub prompt: String,
}
