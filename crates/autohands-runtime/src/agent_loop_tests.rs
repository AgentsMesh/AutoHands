use super::*;
use crate::checkpoint::{CheckpointData, CheckpointSupport};
use crate::memory_persistence;
use async_trait::async_trait;
use autohands_protocols::agent::{AgentConfig, AgentResponse};
use autohands_protocols::tool::AbortSignal;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MockAgent {
    config: AgentConfig,
    complete_immediately: bool,
}

impl MockAgent {
    fn new(complete: bool) -> Self {
        Self {
            config: AgentConfig::new("mock-agent", "Mock Agent", "mock-model"),
            complete_immediately: complete,
        }
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process(
        &self,
        message: Message,
        _ctx: AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        Ok(AgentResponse {
            message: Message::assistant(&format!("Echo: {}", message.content.text())),
            is_complete: self.complete_immediately,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
            usage: None,
        })
    }
}

/// Mock checkpoint support for testing.
struct MockCheckpointSupport {
    interval: u32,
    checkpoint_count: AtomicU32,
    checkpoints: Mutex<Vec<(u32, Vec<Message>)>>,
}

impl MockCheckpointSupport {
    fn new(interval: u32) -> Self {
        Self {
            interval,
            checkpoint_count: AtomicU32::new(0),
            checkpoints: Mutex::new(Vec::new()),
        }
    }

    fn checkpoint_count(&self) -> u32 {
        self.checkpoint_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl CheckpointSupport for MockCheckpointSupport {
    fn should_checkpoint(&self, turn: u32) -> bool {
        turn > 0 && turn % self.interval == 0
    }

    async fn create_checkpoint(
        &self,
        _session_id: &str,
        turn: u32,
        messages: &[Message],
        _context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.checkpoint_count.fetch_add(1, Ordering::SeqCst);
        let mut checkpoints = self.checkpoints.lock().await;
        checkpoints.push((turn, messages.to_vec()));
        Ok(())
    }

    async fn get_latest_checkpoint(
        &self,
        _session_id: &str,
    ) -> Result<Option<CheckpointData>, Box<dyn std::error::Error + Send + Sync>> {
        let checkpoints = self.checkpoints.lock().await;
        if let Some((turn, messages)) = checkpoints.last().cloned() {
            Ok(Some(CheckpointData {
                turn,
                messages,
                context: serde_json::json!({}),
            }))
        } else {
            Ok(None)
        }
    }
}

#[test]
fn test_agent_loop_config_default() {
    let config = AgentLoopConfig::default();
    assert_eq!(config.max_turns, 50);
    assert_eq!(config.timeout_seconds, 300);
    assert!(!config.checkpoint_enabled);
}

#[test]
fn test_agent_loop_config_custom() {
    let config = AgentLoopConfig {
        max_turns: 100,
        timeout_seconds: 600,
        checkpoint_enabled: true,
        ..Default::default()
    };
    assert_eq!(config.max_turns, 100);
    assert_eq!(config.timeout_seconds, 600);
    assert!(config.checkpoint_enabled);
}

#[test]
fn test_agent_loop_config_with_checkpoint() {
    let config = AgentLoopConfig::default().with_checkpoint();
    assert!(config.checkpoint_enabled);
}

#[test]
fn test_agent_loop_config_clone() {
    let config = AgentLoopConfig::default();
    let cloned = config.clone();
    assert_eq!(cloned.max_turns, config.max_turns);
    assert_eq!(cloned.timeout_seconds, config.timeout_seconds);
    assert_eq!(cloned.checkpoint_enabled, config.checkpoint_enabled);
}

#[test]
fn test_agent_loop_config_debug() {
    let config = AgentLoopConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("max_turns"));
    assert!(debug_str.contains("50"));
}

#[test]
fn test_agent_loop_creation() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    let _loop = AgentLoop::new(provider_registry, tool_registry, config);
}

#[test]
fn test_agent_loop_with_custom_config() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig {
        max_turns: 10,
        timeout_seconds: 60,
        checkpoint_enabled: false,
        ..Default::default()
    };

    let _loop = AgentLoop::new(provider_registry, tool_registry, config);
}

#[test]
fn test_agent_loop_config_min_values() {
    let config = AgentLoopConfig {
        max_turns: 1,
        timeout_seconds: 1,
        checkpoint_enabled: false,
        ..Default::default()
    };
    assert_eq!(config.max_turns, 1);
    assert_eq!(config.timeout_seconds, 1);
}

#[test]
fn test_agent_loop_config_max_values() {
    let config = AgentLoopConfig {
        max_turns: u32::MAX,
        timeout_seconds: u64::MAX,
        checkpoint_enabled: true,
        ..Default::default()
    };
    assert_eq!(config.max_turns, u32::MAX);
    assert_eq!(config.timeout_seconds, u64::MAX);
}

#[test]
fn test_agent_loop_config_debug_contains_timeout() {
    let config = AgentLoopConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("timeout_seconds"));
    assert!(debug_str.contains("300"));
}

#[test]
fn test_agent_loop_with_empty_registries() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);
    assert_eq!(agent_loop.config.max_turns, 50);
}

#[tokio::test]
async fn test_agent_loop_run_completes_immediately() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(result.is_ok());
    let messages = result.unwrap();
    assert!(messages.len() >= 2); // At least initial message and response
}

#[tokio::test]
async fn test_agent_loop_run_aborted() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);

    let agent = MockAgent::new(false); // Won't complete, will be aborted

    let abort_signal = Arc::new(AbortSignal::new());
    abort_signal.abort(); // Abort immediately

    let ctx = AgentContext {
        session_id: "test-session".to_string(),
        history: Vec::new(),
        abort_signal,
        data: HashMap::new(),
    };
    let message = Message::user("Hello");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(matches!(result, Err(AgentError::Aborted)));
}

#[tokio::test]
async fn test_agent_loop_run_max_turns_exceeded() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig {
        max_turns: 1,
        timeout_seconds: 60,
        checkpoint_enabled: false,
        ..Default::default()
    };
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);

    let agent = MockAgent::new(false); // Won't complete
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(matches!(result, Err(AgentError::MaxTurnsExceeded(_))));
}

#[tokio::test]
async fn test_agent_loop_execute_tool_not_found() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);

    let tool_call = autohands_protocols::types::ToolCall {
        id: "call_1".to_string(),
        name: "nonexistent_tool".to_string(),
        arguments: serde_json::json!({}),
    };
    let ctx = AgentContext::new("test-session");

    let result = agent_loop.execute_tool(&tool_call, &ctx).await;
    assert!(result.contains("Tool not found"));
}

#[test]
fn test_checkpoint_data_debug() {
    let data = CheckpointData {
        turn: 5,
        messages: vec![Message::user("test")],
        context: serde_json::json!({}),
    };
    let debug_str = format!("{:?}", data);
    assert!(debug_str.contains("turn"));
    assert!(debug_str.contains("5"));
}

#[test]
fn test_checkpoint_data_clone() {
    let data = CheckpointData {
        turn: 5,
        messages: vec![Message::user("test")],
        context: serde_json::json!({"key": "value"}),
    };
    let cloned = data.clone();
    assert_eq!(cloned.turn, data.turn);
    assert_eq!(cloned.messages.len(), data.messages.len());
}

#[test]
fn test_mock_checkpoint_should_checkpoint() {
    let mock = MockCheckpointSupport::new(5);
    assert!(!mock.should_checkpoint(0));
    assert!(!mock.should_checkpoint(3));
    assert!(mock.should_checkpoint(5));
    assert!(mock.should_checkpoint(10));
}

#[tokio::test]
async fn test_mock_checkpoint_create_and_get() {
    let mock = MockCheckpointSupport::new(5);

    let messages = vec![Message::user("test")];
    mock.create_checkpoint("session1", 5, &messages, &serde_json::json!({}))
        .await
        .unwrap();

    assert_eq!(mock.checkpoint_count(), 1);

    let latest = mock.get_latest_checkpoint("session1").await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().turn, 5);
}

#[tokio::test]
async fn test_mock_checkpoint_no_checkpoint() {
    let mock = MockCheckpointSupport::new(5);
    let latest = mock.get_latest_checkpoint("session1").await.unwrap();
    assert!(latest.is_none());
}

#[tokio::test]
async fn test_agent_loop_with_checkpoint_support() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default().with_checkpoint();

    let checkpoint = Arc::new(MockCheckpointSupport::new(1)); // Checkpoint every turn

    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_checkpoint(checkpoint.clone());

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(result.is_ok());

    // Should have created at least one checkpoint
    assert!(checkpoint.checkpoint_count() >= 1);
}

#[tokio::test]
async fn test_agent_loop_run_with_recovery_no_checkpoint() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default().with_checkpoint();

    let checkpoint = Arc::new(MockCheckpointSupport::new(5));

    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_checkpoint(checkpoint);

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    // Should run normally since no checkpoint exists
    let result = agent_loop.run_with_recovery(&agent, ctx, message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_loop_run_with_recovery_from_checkpoint() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default().with_checkpoint();

    let checkpoint = Arc::new(MockCheckpointSupport::new(1));

    // Pre-populate a checkpoint
    checkpoint
        .create_checkpoint(
            "test-session",
            3,
            &[Message::user("recovered"), Message::assistant("test")],
            &serde_json::json!({}),
        )
        .await
        .unwrap();

    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_checkpoint(checkpoint);

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    // Should recover from checkpoint
    let result = agent_loop.run_with_recovery(&agent, ctx, message).await;
    assert!(result.is_ok());

    let messages = result.unwrap();
    // Should contain recovered messages plus new ones
    assert!(messages.len() >= 2);
}

#[tokio::test]
async fn test_agent_loop_no_checkpoint_run_with_recovery() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    // No checkpoint support
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config);

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Hello");

    // Should run normally without checkpoint
    let result = agent_loop.run_with_recovery(&agent, ctx, message).await;
    assert!(result.is_ok());
}

// --- Mock memory backend for testing memory flush / session summary ---

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{MemoryEntry, MemoryQuery as MQuery, MemorySearchResult};

struct MockMemoryBackend {
    stored: Mutex<Vec<MemoryEntry>>,
    search_results: Mutex<Vec<MemorySearchResult>>,
}

impl MockMemoryBackend {
    fn new() -> Self {
        Self {
            stored: Mutex::new(Vec::new()),
            search_results: Mutex::new(Vec::new()),
        }
    }

    fn with_search_results(results: Vec<MemorySearchResult>) -> Self {
        Self {
            stored: Mutex::new(Vec::new()),
            search_results: Mutex::new(results),
        }
    }

    async fn stored_entries(&self) -> Vec<MemoryEntry> {
        self.stored.lock().await.clone()
    }
}

#[async_trait]
impl MemoryBackend for MockMemoryBackend {
    fn id(&self) -> &str {
        "mock-memory"
    }

    async fn store(&self, entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = format!("mem-{}", self.stored.lock().await.len());
        let mut stored = self.stored.lock().await;
        stored.push(entry);
        Ok(id)
    }

    async fn retrieve(&self, _id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        Ok(None)
    }

    async fn search(&self, _query: MQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        Ok(self.search_results.lock().await.clone())
    }

    async fn delete(&self, _id: &str) -> Result<(), MemoryError> {
        Ok(())
    }

    async fn update(&self, _id: &str, _entry: MemoryEntry) -> Result<(), MemoryError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_memory_flush_on_session_complete() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    let memory = Arc::new(MockMemoryBackend::new());
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory.clone());

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    // User message that matches "prefer" keyword -> should be flushed
    let message = Message::user("I prefer using Rust for this project");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(result.is_ok());

    let entries = memory.stored_entries().await;
    // Should have at least 1 flushed entry (preference) + 1 session summary
    assert!(entries.len() >= 2, "Expected >=2 stored entries, got {}", entries.len());

    // Verify the preference entry
    let pref_entry = entries.iter().find(|e| e.memory_type == "preference");
    assert!(pref_entry.is_some(), "Expected a preference entry to be flushed");
    assert!(pref_entry.unwrap().tags.contains(&"session-end-flush".to_string()));

    // Verify the session summary entry
    let summary_entry = entries.iter().find(|e| e.memory_type == "conversation");
    assert!(summary_entry.is_some(), "Expected a session summary entry");
    assert!(summary_entry.unwrap().tags.contains(&"session-summary".to_string()));
    assert!(summary_entry.unwrap().content.contains("Session conversation summary"));
}

#[tokio::test]
async fn test_memory_flush_on_max_turns() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig {
        max_turns: 1,
        timeout_seconds: 60,
        checkpoint_enabled: false,
        ..Default::default()
    };

    let memory = Arc::new(MockMemoryBackend::new());
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory.clone());

    let agent = MockAgent::new(false); // won't complete -> triggers max_turns
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("I decided to use PostgreSQL for the database");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(matches!(result, Err(AgentError::MaxTurnsExceeded(_))));

    let entries = memory.stored_entries().await;
    // Should have flushed the decision entry with session-end-flush tag
    let decision_entry = entries.iter().find(|e| e.memory_type == "decision");
    assert!(decision_entry.is_some(), "Expected a decision entry on max_turns flush");
    assert!(decision_entry.unwrap().tags.contains(&"session-end-flush".to_string()));
}

#[tokio::test]
async fn test_memory_flush_scans_assistant_messages() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();
    let memory_concrete = Arc::new(MockMemoryBackend::new());
    let memory_dyn: Arc<dyn MemoryBackend> = memory_concrete.clone();

    // Test flush_memories_to_backend directly via the free function
    let _agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory_dyn.clone());

    let messages = vec![
        Message::user("What should we use?"),
        Message::assistant("The plan is to use Redis for caching and PostgreSQL for persistence"),
    ];

    memory_persistence::flush_memories_to_backend(&messages, &memory_dyn, "session-end-flush")
        .await;

    let entries = memory_concrete.stored_entries().await;
    // The assistant message contains "the plan is" -> decision type
    let decision = entries.iter().find(|e| e.memory_type == "decision");
    assert!(decision.is_some(), "Expected assistant decision to be captured");
}

#[tokio::test]
async fn test_memory_no_flush_on_abort() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    let memory = Arc::new(MockMemoryBackend::new());
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory.clone());

    let agent = MockAgent::new(false);
    let abort_signal = Arc::new(AbortSignal::new());
    abort_signal.abort();

    let ctx = AgentContext {
        session_id: "test-session".to_string(),
        history: Vec::new(),
        abort_signal,
        data: HashMap::new(),
    };
    let message = Message::user("I prefer Python");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(matches!(result, Err(AgentError::Aborted)));

    // Abort should NOT flush memory
    let entries = memory.stored_entries().await;
    assert!(entries.is_empty(), "Expected no memory flush on abort");
}

#[tokio::test]
async fn test_memory_context_injection_on_run() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();

    let search_results = vec![MemorySearchResult {
        entry: MemoryEntry::new("User prefers Rust", "preference"),
        relevance: 0.9,
    }];
    let memory = Arc::new(MockMemoryBackend::with_search_results(search_results));
    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory.clone());

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Help me set up the project");

    let result = agent_loop.run(&agent, ctx, message).await;
    assert!(result.is_ok());

    let messages = result.unwrap();
    // The first message should be the injected system memory context
    let has_memory_msg = messages.iter().any(|m| {
        matches!(m.role, autohands_protocols::types::MessageRole::System)
            && m.content.text().contains("relevant memories")
    });
    assert!(has_memory_msg, "Expected memory context injection as system message");
}

#[tokio::test]
async fn test_session_summary_content() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default();
    let memory_concrete = Arc::new(MockMemoryBackend::new());
    let memory_dyn: Arc<dyn MemoryBackend> = memory_concrete.clone();

    let _agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_memory(memory_dyn.clone());

    let messages = vec![
        Message::user("How do I configure Redis?"),
        Message::assistant("You can configure Redis by editing redis.conf"),
        Message::user("What about clustering?"),
        Message::assistant("Redis Cluster provides automatic sharding"),
    ];

    memory_persistence::store_session_summary(&messages, "sess-123", &memory_dyn)
        .await;

    let entries = memory_concrete.stored_entries().await;
    assert_eq!(entries.len(), 1);

    let summary = &entries[0];
    assert_eq!(summary.memory_type, "conversation");
    assert!(summary.content.contains("Session conversation summary"));
    assert!(summary.content.contains("How do I configure Redis?"));
    assert!(summary.content.contains("What about clustering?"));
    assert!(summary.tags.contains(&"session-summary".to_string()));
    assert!(summary.tags.contains(&"session:sess-123".to_string()));
    assert_eq!(summary.importance, Some(0.4));
}

#[tokio::test]
async fn test_run_from_turn_with_memory_integration() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentLoopConfig::default().with_checkpoint();

    let search_results = vec![MemorySearchResult {
        entry: MemoryEntry::new("Previous session: user prefers Rust", "preference"),
        relevance: 0.85,
    }];
    let memory = Arc::new(MockMemoryBackend::with_search_results(search_results));
    let checkpoint = Arc::new(MockCheckpointSupport::new(1));

    // Pre-populate a checkpoint with messages containing a preference keyword
    checkpoint
        .create_checkpoint(
            "test-session",
            3,
            &[
                Message::user("I always use cargo for Rust projects"),
                Message::assistant("Great choice!"),
            ],
            &serde_json::json!({}),
        )
        .await
        .unwrap();

    let agent_loop = AgentLoop::new(provider_registry, tool_registry, config)
        .with_checkpoint(checkpoint)
        .with_memory(memory.clone());

    let agent = MockAgent::new(true);
    let ctx = AgentContext::new("test-session").with_history(Vec::new());
    let message = Message::user("Continue");

    // run_with_recovery -> triggers run_from_turn
    let result = agent_loop.run_with_recovery(&agent, ctx, message).await;
    assert!(result.is_ok());

    let messages = result.unwrap();
    // Should have injected memory context (system message) at the beginning
    let has_memory_injection = messages.iter().any(|m| {
        matches!(m.role, autohands_protocols::types::MessageRole::System)
            && m.content.text().contains("relevant memories")
    });
    assert!(has_memory_injection, "Expected memory injection in run_from_turn");

    // Should have stored entries: flush + session summary
    let entries = memory.stored_entries().await;
    assert!(entries.len() >= 1, "Expected memory entries from run_from_turn");

    // Verify session summary exists
    let has_summary = entries.iter().any(|e| e.tags.contains(&"session-summary".to_string()));
    assert!(has_summary, "Expected session summary from run_from_turn");
}
