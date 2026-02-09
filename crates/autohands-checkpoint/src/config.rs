//! Checkpoint configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Checkpoint configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Whether checkpointing is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Checkpoint after every N turns.
    #[serde(default = "default_interval")]
    pub interval_turns: u32,

    /// Storage path for checkpoints.
    #[serde(default = "default_storage_path")]
    pub storage_path: PathBuf,

    /// Maximum number of checkpoints to keep.
    #[serde(default = "default_max_checkpoints")]
    pub max_checkpoints: u32,

    /// Auto-recovery on startup.
    #[serde(default = "default_auto_recover")]
    pub auto_recover: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_interval() -> u32 {
    5
}

fn default_storage_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("checkpoints"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands/checkpoints"))
}

fn default_max_checkpoints() -> u32 {
    10
}

fn default_auto_recover() -> bool {
    true
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            interval_turns: default_interval(),
            storage_path: default_storage_path(),
            max_checkpoints: default_max_checkpoints(),
            auto_recover: default_auto_recover(),
        }
    }
}
