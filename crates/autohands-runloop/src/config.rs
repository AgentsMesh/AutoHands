//! Configuration for the RunLoop.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// RunLoop configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLoopConfig {
    /// Worker pool configuration.
    #[serde(default)]
    pub workers: WorkerPoolConfig,

    /// Task queue configuration.
    #[serde(default)]
    pub queue: TaskQueueConfig,

    /// Task chain configuration.
    #[serde(default)]
    pub chain: TaskChainConfig,

    /// Retry configuration.
    #[serde(default)]
    pub retry: RetryConfig,

    /// Whether to enable metrics collection.
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,

    /// Checkpoint interval in seconds (0 = disabled).
    #[serde(default)]
    pub checkpoint_interval_secs: u64,
}

fn default_metrics_enabled() -> bool {
    true
}

impl Default for RunLoopConfig {
    fn default() -> Self {
        Self {
            workers: WorkerPoolConfig::default(),
            queue: TaskQueueConfig::default(),
            chain: TaskChainConfig::default(),
            retry: RetryConfig::default(),
            metrics_enabled: true,
            checkpoint_interval_secs: 60,
        }
    }
}

/// Worker pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolConfig {
    /// Maximum number of concurrent workers.
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
}

fn default_max_workers() -> usize {
    4
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            max_workers: default_max_workers(),
        }
    }
}

/// Task queue configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueConfig {
    /// Maximum number of pending tasks.
    #[serde(default = "default_max_pending_tasks")]
    pub max_pending_tasks: usize,

    /// Path for task persistence (empty = in-memory only).
    #[serde(default)]
    pub persist_path: Option<String>,
}

fn default_max_pending_tasks() -> usize {
    10000
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_pending_tasks: default_max_pending_tasks(),
            persist_path: None,
        }
    }
}

/// Task chain configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskChainConfig {
    /// Maximum tasks per correlation chain.
    #[serde(default = "default_max_tasks_per_chain")]
    pub max_tasks_per_chain: u32,
}

fn default_max_tasks_per_chain() -> u32 {
    100
}

impl Default for TaskChainConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_chain: default_max_tasks_per_chain(),
        }
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Delay between retries in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
        }
    }
}

impl RetryConfig {
    /// Get retry delay as Duration.
    pub fn retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RunLoopConfig::default();
        assert_eq!(config.workers.max_workers, 4);
        assert_eq!(config.queue.max_pending_tasks, 10000);
        assert_eq!(config.chain.max_tasks_per_chain, 100);
        assert_eq!(config.retry.max_retries, 3);
        assert!(config.metrics_enabled);
    }

    #[test]
    fn test_retry_delay() {
        let config = RetryConfig {
            max_retries: 3,
            retry_delay_ms: 500,
        };
        assert_eq!(config.retry_delay(), Duration::from_millis(500));
    }

    #[test]
    fn test_config_serialization() {
        let config = RunLoopConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: RunLoopConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workers.max_workers, config.workers.max_workers);
    }
}
