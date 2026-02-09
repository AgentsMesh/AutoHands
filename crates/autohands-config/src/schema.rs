//! Configuration schema definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

// ============================================================================
// 24/7 Autonomous Agent Configuration Structures
// ============================================================================

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

fn default_true() -> bool {
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.agent.max_turns, 50);
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_agent_config_default() {
        let agent = AgentConfig::default();
        assert_eq!(agent.default, "general");
        assert_eq!(agent.max_turns, 50);
        assert_eq!(agent.timeout_seconds, 300);
    }

    #[test]
    fn test_memory_config_default() {
        let memory = MemoryConfig::default();
        assert_eq!(memory.backend, "markdown");
        assert!(memory.path.is_none());
        // Test persistent config defaults
        assert!(memory.persistent.enabled);
        assert_eq!(memory.persistent.max_memories, 1000);
        assert!(memory.persistent.auto_cleanup);
    }

    #[test]
    fn test_extensions_config_default() {
        let extensions = ExtensionsConfig::default();
        assert!(extensions.paths.is_empty());
        assert!(extensions.enabled.is_empty());
        assert!(extensions.disabled.is_empty());
    }

    #[test]
    fn test_skills_config_default() {
        let skills = SkillsConfig::default();
        assert!(skills.paths.is_empty());
        assert!(skills.enabled.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("127.0.0.1"));
        assert!(json.contains("8080"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "server": {"host": "0.0.0.0", "port": 3000},
            "agent": {"default": "custom", "max_turns": 100, "timeout_seconds": 600}
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.agent.default, "custom");
        assert_eq!(config.agent.max_turns, 100);
    }

    #[test]
    fn test_provider_config_serialization() {
        let provider = ProviderConfig {
            api_key: Some("test-key".to_string()),
            base_url: Some("https://api.example.com".to_string()),
            default_model: Some("gpt-4".to_string()),
            extra: HashMap::new(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("https://api.example.com"));
    }

    #[test]
    fn test_provider_config_skip_serializing_none() {
        let provider = ProviderConfig {
            api_key: None,
            base_url: None,
            default_model: None,
            extra: HashMap::new(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        // None fields should be skipped
        assert!(!json.contains("api_key"));
        assert!(!json.contains("base_url"));
    }

    #[test]
    fn test_memory_config_with_path() {
        let json = r#"{"backend": "sqlite", "path": "/tmp/memory.db"}"#;
        let memory: MemoryConfig = serde_json::from_str(json).unwrap();
        assert_eq!(memory.backend, "sqlite");
        assert_eq!(memory.path.unwrap().to_str().unwrap(), "/tmp/memory.db");
    }

    #[test]
    fn test_extensions_config_with_paths() {
        let json = r#"{"paths": ["/ext1", "/ext2"], "enabled": ["a", "b"], "disabled": ["c"]}"#;
        let ext: ExtensionsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ext.paths.len(), 2);
        assert_eq!(ext.enabled.len(), 2);
        assert_eq!(ext.disabled.len(), 1);
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(cloned.server.host, config.server.host);
        assert_eq!(cloned.server.port, config.server.port);
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("Config"));
        assert!(debug.contains("server"));
    }

    #[test]
    fn test_provider_config_extra() {
        let mut provider = ProviderConfig {
            api_key: Some("key".to_string()),
            base_url: None,
            default_model: None,
            extra: HashMap::new(),
        };
        provider.extra.insert("custom".to_string(), serde_json::json!("value"));

        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("custom"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 9000

            [agent]
            default = "test"
            max_turns = 25
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.agent.default, "test");
    }

    #[test]
    fn test_partial_config_deserialization() {
        let json = r#"{"server": {"port": 5000}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        // Should use default for host
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 5000);
    }

    // Tests for 24/7 Autonomous Agent configurations

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(!config.enabled); // Disabled by default
        assert!(config.auto_restart);
        assert_eq!(config.max_restarts, 10);
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timezone, "UTC");
        assert!(config.jobs.is_empty());
    }

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.max_retries, 3);
        assert!(config.dead_letter_queue_enabled);
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_turns, 5);
        assert_eq!(config.max_checkpoints, 10);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_concurrent_workflows, 5);
        assert_eq!(config.default_step_timeout_secs, 300);
    }

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.health_endpoint, "/health");
        assert_eq!(config.metrics_endpoint, "/metrics");
    }

    #[test]
    fn test_full_config_with_24_7_features() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 8080

            [daemon]
            enabled = true
            auto_restart = true
            max_restarts = 5

            [scheduler]
            enabled = true
            timezone = "Asia/Shanghai"

            [[scheduler.jobs]]
            id = "daily-report"
            schedule = "0 0 9 * * *"
            agent = "general"
            prompt = "Generate daily report"

            [queue]
            max_workers = 8
            max_retries = 5

            [checkpoint]
            enabled = true
            interval_turns = 10

            [orchestrator]
            enabled = true
            max_concurrent_workflows = 10

            [monitor]
            enabled = true
            health_endpoint = "/api/health"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.daemon.enabled);
        assert_eq!(config.daemon.max_restarts, 5);
        assert_eq!(config.scheduler.timezone, "Asia/Shanghai");
        assert_eq!(config.scheduler.jobs.len(), 1);
        assert_eq!(config.scheduler.jobs[0].id, "daily-report");
        assert_eq!(config.queue.max_workers, 8);
        assert_eq!(config.checkpoint.interval_turns, 10);
        assert_eq!(config.orchestrator.max_concurrent_workflows, 10);
        assert_eq!(config.monitor.health_endpoint, "/api/health");
    }
}
