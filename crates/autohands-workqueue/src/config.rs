//! Queue configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Queue configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum number of concurrent workers.
    #[serde(default = "default_max_workers")]
    pub max_workers: u32,

    /// Maximum retries for failed tasks.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry delay in seconds.
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,

    /// Maximum queue size (0 = unlimited).
    #[serde(default)]
    pub max_queue_size: u64,

    /// Database path for task persistence.
    #[serde(default)]
    pub db_path: Option<PathBuf>,

    /// Dead letter queue enabled.
    #[serde(default = "default_dlq_enabled")]
    pub dead_letter_queue_enabled: bool,
}

fn default_max_workers() -> u32 {
    4
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    5
}

fn default_dlq_enabled() -> bool {
    true
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_workers: default_max_workers(),
            max_retries: default_max_retries(),
            retry_delay_secs: default_retry_delay(),
            max_queue_size: 0,
            db_path: None,
            dead_letter_queue_enabled: default_dlq_enabled(),
        }
    }
}
