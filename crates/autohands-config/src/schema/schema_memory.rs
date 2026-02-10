//! Memory-related configuration types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory backend type ("markdown", "sqlite", etc.).
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Legacy path option (use persistent.storage_path instead).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    /// Persistent storage configuration.
    #[serde(default)]
    pub persistent: PersistentMemoryConfig,
}

/// Persistent memory storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentMemoryConfig {
    /// Whether persistent memory storage is enabled.
    #[serde(default = "default_persistent_enabled")]
    pub enabled: bool,

    /// Storage path for memory files.
    #[serde(default = "default_persistent_storage_path")]
    pub storage_path: PathBuf,

    /// Maximum number of memories to keep per type.
    #[serde(default = "default_max_memories")]
    pub max_memories: u32,

    /// Auto-cleanup old memories.
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup: bool,
}

impl Default for PersistentMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_persistent_enabled(),
            storage_path: default_persistent_storage_path(),
            max_memories: default_max_memories(),
            auto_cleanup: default_auto_cleanup(),
        }
    }
}

fn default_persistent_enabled() -> bool {
    true
}

fn default_persistent_storage_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".autohands")
        .join("memory")
}

fn default_max_memories() -> u32 {
    1000
}

fn default_auto_cleanup() -> bool {
    true
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            path: None,
            persistent: PersistentMemoryConfig::default(),
        }
    }
}

fn default_backend() -> String {
    "markdown".to_string()
}
