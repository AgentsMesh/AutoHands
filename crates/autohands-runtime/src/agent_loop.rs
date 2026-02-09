//! Agentic loop implementation.

use std::sync::Arc;

use tracing::{debug, info, warn};

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::{Agent, AgentContext};
use autohands_protocols::error::AgentError;
use autohands_protocols::tool::ToolContext;
use autohands_protocols::types::Message;

use crate::transcript::TranscriptWriter;

/// Checkpoint support trait (optional integration).
#[async_trait::async_trait]
pub trait CheckpointSupport: Send + Sync {
    /// Check if a checkpoint should be created at this turn.
    fn should_checkpoint(&self, turn: u32) -> bool;

    /// Create a checkpoint with the current state.
    async fn create_checkpoint(
        &self,
        session_id: &str,
        turn: u32,
        messages: &[Message],
        context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the latest checkpoint for recovery.
    async fn get_latest_checkpoint(
        &self,
        session_id: &str,
    ) -> Result<Option<CheckpointData>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Checkpoint data for recovery.
#[derive(Debug, Clone)]
pub struct CheckpointData {
    /// Turn number when checkpoint was created.
    pub turn: u32,
    /// Serialized messages.
    pub messages: Vec<Message>,
    /// Serialized context.
    pub context: serde_json::Value,
}

/// Configuration for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    pub max_turns: u32,
    pub timeout_seconds: u64,
    /// Enable checkpoint support.
    pub checkpoint_enabled: bool,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            timeout_seconds: 300,
            checkpoint_enabled: false,
        }
    }
}

impl AgentLoopConfig {
    /// Create a new config with checkpoint support enabled.
    pub fn with_checkpoint(mut self) -> Self {
        self.checkpoint_enabled = true;
        self
    }
}

/// The agentic loop executor.
pub struct AgentLoop {
    #[allow(dead_code)]
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    config: AgentLoopConfig,
    checkpoint: Option<Arc<dyn CheckpointSupport>>,
    transcript: Option<Arc<TranscriptWriter>>,
}

impl AgentLoop {
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        config: AgentLoopConfig,
    ) -> Self {
        Self {
            provider_registry,
            tool_registry,
            config,
            checkpoint: None,
            transcript: None,
        }
    }

    /// Set checkpoint support.
    pub fn with_checkpoint(mut self, checkpoint: Arc<dyn CheckpointSupport>) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Set transcript writer for session recording.
    pub fn with_transcript(mut self, transcript: Option<Arc<TranscriptWriter>>) -> Self {
        self.transcript = transcript;
        self
    }

    /// Get the transcript writer (for passing to agent executor).
    pub fn transcript(&self) -> Option<Arc<TranscriptWriter>> {
        self.transcript.clone()
    }

    /// Run the agent loop.
    pub async fn run(
        &self,
        agent: &dyn Agent,
        mut ctx: AgentContext,
        initial_message: Message,
    ) -> Result<Vec<Message>, AgentError> {
        let start_time = std::time::Instant::now();
        let mut messages = ctx.history.clone();
        messages.push(initial_message.clone());

        // Record initial user message to transcript
        if let Some(ref transcript) = self.transcript {
            let content = serde_json::to_value(&initial_message.content).unwrap_or_default();
            if let Err(e) = transcript.record_user_message(content).await {
                warn!("Failed to record user message to transcript: {}", e);
            }
        }

        let mut turn = 0;

        loop {
            if ctx.abort_signal.is_aborted() {
                self.record_session_end("aborted", Some("User aborted"), turn, &start_time).await;
                return Err(AgentError::Aborted);
            }

            if turn >= self.config.max_turns {
                self.record_session_end("max_turns", Some("Max turns exceeded"), turn, &start_time).await;
                return Err(AgentError::MaxTurnsExceeded(turn));
            }

            turn += 1;
            debug!("Agent loop turn {}", turn);

            // Process through agent
            ctx.history = messages.clone();
            let response = agent
                .process(messages.last().unwrap().clone(), ctx.clone())
                .await?;

            // Record assistant message to transcript
            if let Some(ref transcript) = self.transcript {
                let content = serde_json::to_value(&response.message.content).unwrap_or_default();
                if let Err(e) = transcript.record_assistant_message(content, None).await {
                    warn!("Failed to record assistant message to transcript: {}", e);
                }
            }

            messages.push(response.message.clone());

            // Create checkpoint if enabled and interval reached
            if let Some(ref checkpoint) = self.checkpoint {
                if checkpoint.should_checkpoint(turn) {
                    let context_data = serde_json::json!({
                        "session_id": ctx.session_id,
                        "data": ctx.data,
                    });

                    if let Err(e) = checkpoint
                        .create_checkpoint(&ctx.session_id, turn, &messages, &context_data)
                        .await
                    {
                        warn!("Failed to create checkpoint at turn {}: {}", turn, e);
                    } else {
                        debug!("Checkpoint created at turn {}", turn);
                    }
                }
            }

            if response.is_complete {
                info!("Agent completed after {} turns", turn);
                self.record_session_end("completed", None, turn, &start_time).await;
                break;
            }

            // Handle tool calls
            for tool_call in &response.tool_calls {
                // Record tool use to transcript
                if let Some(ref transcript) = self.transcript {
                    if let Err(e) = transcript.record_tool_use(
                        &tool_call.id,
                        &tool_call.name,
                        tool_call.arguments.clone(),
                    ).await {
                        warn!("Failed to record tool use to transcript: {}", e);
                    }
                }

                let tool_start = std::time::Instant::now();
                let result = self.execute_tool(tool_call, &ctx).await;
                let duration_ms = tool_start.elapsed().as_millis() as u64;

                // Record tool result to transcript
                if let Some(ref transcript) = self.transcript {
                    let is_error = result.starts_with("Error:");
                    if let Err(e) = transcript.record_tool_result(
                        &tool_call.id,
                        &tool_call.name,
                        !is_error,
                        Some(&result),
                        if is_error { Some(&result) } else { None },
                        Some(duration_ms),
                    ).await {
                        warn!("Failed to record tool result to transcript: {}", e);
                    }
                }

                let tool_message = Message::tool(&tool_call.id, result);
                messages.push(tool_message);
            }
        }

        Ok(messages)
    }

    /// Record session end to transcript.
    async fn record_session_end(&self, status: &str, error: Option<&str>, turns: u32, start_time: &std::time::Instant) {
        if let Some(ref transcript) = self.transcript {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            if let Err(e) = transcript.record_session_end(status, error, turns, Some(duration_ms)).await {
                warn!("Failed to record session end to transcript: {}", e);
            }
        }
    }

    /// Run the agent loop with recovery from checkpoint.
    pub async fn run_with_recovery(
        &self,
        agent: &dyn Agent,
        mut ctx: AgentContext,
        initial_message: Message,
    ) -> Result<Vec<Message>, AgentError> {
        // Check for existing checkpoint
        if let Some(ref checkpoint) = self.checkpoint {
            match checkpoint.get_latest_checkpoint(&ctx.session_id).await {
                Ok(Some(cp_data)) => {
                    info!(
                        "Recovering from checkpoint at turn {} with {} messages",
                        cp_data.turn,
                        cp_data.messages.len()
                    );

                    // Restore context data from checkpoint
                    if let Some(data) = cp_data.context.get("data") {
                        if let Ok(restored_data) = serde_json::from_value(data.clone()) {
                            ctx.data = restored_data;
                        }
                    }

                    // Resume from checkpoint
                    return self.run_from_turn(agent, ctx, cp_data.messages, cp_data.turn).await;
                }
                Ok(None) => {
                    debug!("No checkpoint found, starting fresh");
                }
                Err(e) => {
                    warn!("Failed to check for checkpoint: {}, starting fresh", e);
                }
            }
        }

        // No checkpoint, run normally
        self.run(agent, ctx, initial_message).await
    }

    /// Run from a specific turn with existing messages.
    async fn run_from_turn(
        &self,
        agent: &dyn Agent,
        mut ctx: AgentContext,
        mut messages: Vec<Message>,
        start_turn: u32,
    ) -> Result<Vec<Message>, AgentError> {
        let mut turn = start_turn;

        loop {
            if ctx.abort_signal.is_aborted() {
                return Err(AgentError::Aborted);
            }

            if turn >= self.config.max_turns {
                return Err(AgentError::MaxTurnsExceeded(turn));
            }

            turn += 1;
            debug!("Agent loop turn {} (resumed)", turn);

            // Process through agent
            ctx.history = messages.clone();
            let response = agent
                .process(messages.last().unwrap().clone(), ctx.clone())
                .await?;

            messages.push(response.message.clone());

            // Create checkpoint if enabled and interval reached
            if let Some(ref checkpoint) = self.checkpoint {
                if checkpoint.should_checkpoint(turn) {
                    let context_data = serde_json::json!({
                        "session_id": ctx.session_id,
                        "data": ctx.data,
                    });

                    if let Err(e) = checkpoint
                        .create_checkpoint(&ctx.session_id, turn, &messages, &context_data)
                        .await
                    {
                        warn!("Failed to create checkpoint at turn {}: {}", turn, e);
                    } else {
                        debug!("Checkpoint created at turn {}", turn);
                    }
                }
            }

            if response.is_complete {
                info!("Agent completed after {} turns", turn);
                break;
            }

            // Handle tool calls
            for tool_call in &response.tool_calls {
                let result = self.execute_tool(tool_call, &ctx).await;
                let tool_message = Message::tool(&tool_call.id, result);
                messages.push(tool_message);
            }
        }

        Ok(messages)
    }

    async fn execute_tool(
        &self,
        tool_call: &autohands_protocols::types::ToolCall,
        ctx: &AgentContext,
    ) -> String {
        let tool = match self.tool_registry.get(&tool_call.name) {
            Some(t) => t,
            None => return format!("Tool not found: {}", tool_call.name),
        };

        let tool_ctx = ToolContext::new(&ctx.session_id, std::env::current_dir().unwrap());

        match tool.execute(tool_call.arguments.clone(), tool_ctx).await {
            Ok(result) => result.content,
            Err(e) => format!("Tool error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::agent::{AgentConfig, AgentResponse};
    use autohands_protocols::tool::AbortSignal;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
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
        };

        let _loop = AgentLoop::new(provider_registry, tool_registry, config);
    }

    #[test]
    fn test_agent_loop_config_min_values() {
        let config = AgentLoopConfig {
            max_turns: 1,
            timeout_seconds: 1,
            checkpoint_enabled: false,
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
}
