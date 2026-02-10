//! Configuration schema definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

mod schema_infra;
mod schema_memory;
mod schema_triggers;

pub use schema_infra::*;
pub use schema_memory::*;
pub use schema_triggers::*;

/// Shared default helper used by submodules.
pub(crate) fn default_true() -> bool {
    true
}

/// Root configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub extensions: ExtensionsConfig,

    #[serde(default)]
    pub skills: SkillsConfig,

    // 24/7 Autonomous Agent configurations
    #[serde(default)]
    pub daemon: DaemonConfig,

    #[serde(default)]
    pub scheduler: SchedulerConfig,

    #[serde(default)]
    pub queue: QueueConfig,

    #[serde(default)]
    pub checkpoint: CheckpointConfig,

    #[serde(default)]
    pub orchestrator: OrchestratorConfig,

    #[serde(default)]
    pub triggers: TriggersConfig,

    #[serde(default)]
    pub monitor: MonitorConfig,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_agent")]
    pub default: String,

    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            default: default_agent(),
            max_turns: default_max_turns(),
            timeout_seconds: default_timeout(),
        }
    }
}

fn default_agent() -> String {
    "general".to_string()
}

fn default_max_turns() -> u32 {
    50
}

fn default_timeout() -> u64 {
    300
}

/// Provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Extensions configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionsConfig {
    #[serde(default)]
    pub paths: Vec<PathBuf>,

    #[serde(default)]
    pub enabled: Vec<String>,

    #[serde(default)]
    pub disabled: Vec<String>,
}

/// Skills configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Additional skill directories.
    #[serde(default)]
    pub paths: Vec<PathBuf>,

    /// Enabled skill IDs.
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Enable hot-reload for dynamic skills.
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,

    /// Use managed skills directory (~/.autohands/skills/).
    #[serde(default = "default_true")]
    pub use_managed: bool,

    /// Use workspace skills directory (<cwd>/skills/).
    #[serde(default = "default_true")]
    pub use_workspace: bool,
}

fn default_hot_reload() -> bool {
    true
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            enabled: Vec::new(),
            hot_reload: default_hot_reload(),
            use_managed: default_true(),
            use_workspace: default_true(),
        }
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
