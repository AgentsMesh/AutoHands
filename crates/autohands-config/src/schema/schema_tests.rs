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
