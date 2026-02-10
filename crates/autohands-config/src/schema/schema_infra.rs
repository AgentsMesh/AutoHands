//! Infrastructure configuration types (scheduler, queue, checkpoint, orchestrator, monitor).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::default_true;

/// Scheduler configuration for cron jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Whether scheduler is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Timezone for cron expressions.
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// Scheduled jobs.
    #[serde(default)]
    pub jobs: Vec<ScheduledJob>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            timezone: default_timezone(),
            jobs: Vec::new(),
        }
    }
}

/// A scheduled job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Unique job ID.
    pub id: String,
    /// Cron schedule expression (6 fields including seconds).
    pub schedule: String,
    /// Agent to execute the job.
    pub agent: String,
    /// Prompt to execute.
    pub prompt: String,
}

/// Queue configuration for task processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum number of concurrent workers.
    #[serde(default = "default_max_workers")]
    pub max_workers: u32,

    /// Maximum retries for failed tasks.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Whether dead letter queue is enabled.
    #[serde(default = "default_true")]
    pub dead_letter_queue_enabled: bool,
}

fn default_max_workers() -> u32 {
    4
}

fn default_max_retries() -> u32 {
    3
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_workers: default_max_workers(),
            max_retries: default_max_retries(),
            dead_letter_queue_enabled: default_true(),
        }
    }
}

/// Checkpoint configuration for recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Whether checkpointing is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Checkpoint after every N turns.
    #[serde(default = "default_interval_turns")]
    pub interval_turns: u32,

    /// Storage path for checkpoints.
    #[serde(default)]
    pub storage_path: Option<PathBuf>,

    /// Maximum number of checkpoints to keep.
    #[serde(default = "default_max_checkpoints")]
    pub max_checkpoints: u32,
}

fn default_interval_turns() -> u32 {
    5
}

fn default_max_checkpoints() -> u32 {
    10
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            interval_turns: default_interval_turns(),
            storage_path: None,
            max_checkpoints: default_max_checkpoints(),
        }
    }
}

/// Orchestrator configuration for multi-agent workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Whether orchestrator is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum concurrent workflows.
    #[serde(default = "default_max_concurrent_workflows")]
    pub max_concurrent_workflows: u32,

    /// Default step timeout in seconds.
    #[serde(default = "default_step_timeout")]
    pub default_step_timeout_secs: u64,
}

fn default_max_concurrent_workflows() -> u32 {
    5
}

fn default_step_timeout() -> u64 {
    300
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_concurrent_workflows: default_max_concurrent_workflows(),
            default_step_timeout_secs: default_step_timeout(),
        }
    }
}

/// Monitor configuration for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Whether monitoring is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Health endpoint path.
    #[serde(default = "default_health_endpoint")]
    pub health_endpoint: String,

    /// Metrics endpoint path.
    #[serde(default = "default_metrics_endpoint")]
    pub metrics_endpoint: String,
}

fn default_health_endpoint() -> String {
    "/health".to_string()
}

fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            health_endpoint: default_health_endpoint(),
            metrics_endpoint: default_metrics_endpoint(),
        }
    }
}
