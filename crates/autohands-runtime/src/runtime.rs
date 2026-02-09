//! Agent runtime for managing agent execution.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::{Agent, AgentConfig, AgentContext};
use autohands_protocols::error::AgentError;
use autohands_protocols::tool::AbortSignal;
use autohands_protocols::types::Message;

use crate::agent_loop::{AgentLoop, AgentLoopConfig};
use crate::history::HistoryManager;
use crate::session::SessionManager;
use crate::transcript::TranscriptWriter;

/// Configuration for the agent runtime.
#[derive(Debug, Clone)]
pub struct AgentRuntimeConfig {
    /// Maximum concurrent agent executions.
    pub max_concurrent: usize,

    /// Default agent loop config.
    pub default_loop_config: AgentLoopConfig,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            default_loop_config: AgentLoopConfig::default(),
        }
    }
}

/// Agent execution handle for tracking running agents.
pub struct AgentHandle {
    /// Session ID.
    pub session_id: String,

    /// Abort signal for cancellation.
    pub abort_signal: Arc<AbortSignal>,
}

/// The agent runtime manages agent execution.
pub struct AgentRuntime {
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    session_manager: Arc<SessionManager>,
    history_manager: Arc<HistoryManager>,
    agents: DashMap<String, Arc<dyn Agent>>,
    running: DashMap<String, AgentHandle>,
    concurrency_semaphore: Arc<Semaphore>,
    config: AgentRuntimeConfig,
}

impl AgentRuntime {
    /// Create a new agent runtime.
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        config: AgentRuntimeConfig,
    ) -> Self {
        Self {
            provider_registry,
            tool_registry,
            session_manager: Arc::new(SessionManager::new()),
            history_manager: Arc::new(HistoryManager::new()),
            agents: DashMap::new(),
            running: DashMap::new(),
            concurrency_semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
            config,
        }
    }

    /// Get history manager.
    pub fn history_manager(&self) -> &Arc<HistoryManager> {
        &self.history_manager
    }

    /// Register an agent.
    pub fn register_agent(&self, agent: Arc<dyn Agent>) {
        let id = agent.id().to_string();
        info!("Registering agent: {}", id);
        self.agents.insert(id, agent);
    }

    /// Unregister an agent.
    pub fn unregister_agent(&self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    /// Get a registered agent.
    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(agent_id).map(|a| a.clone())
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<AgentConfig> {
        self.agents.iter().map(|a| a.config().clone()).collect()
    }

    /// Execute an agent with a message.
    pub async fn execute(
        &self,
        agent_id: &str,
        session_id: &str,
        message: Message,
    ) -> Result<Vec<Message>, AgentError> {
        self.execute_with_transcript(agent_id, session_id, message, None).await
    }

    /// Execute an agent with a message and optional transcript recording.
    pub async fn execute_with_transcript(
        &self,
        agent_id: &str,
        session_id: &str,
        message: Message,
        transcript: Option<Arc<TranscriptWriter>>,
    ) -> Result<Vec<Message>, AgentError> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| AgentError::NotFound(agent_id.to_string()))?
            .clone();

        // Acquire semaphore permit for concurrency control
        let _permit = self.concurrency_semaphore.acquire().await.map_err(|_| {
            AgentError::ExecutionFailed("Failed to acquire concurrency permit".to_string())
        })?;

        // Create abort signal
        let abort_signal = Arc::new(AbortSignal::new());

        // Register as running
        self.running.insert(
            session_id.to_string(),
            AgentHandle {
                session_id: session_id.to_string(),
                abort_signal: abort_signal.clone(),
            },
        );

        // Get conversation history for this session
        let history = self.history_manager.get(session_id);
        let history_messages = history.messages().to_vec();

        // Create context with history from HistoryManager
        let ctx = AgentContext::new(session_id).with_history(history_messages);
        let ctx = AgentContext {
            abort_signal,
            ..ctx
        };

        // Record user message to history
        self.history_manager.push(session_id, message.clone());

        // Create and run agent loop with transcript
        let agent_loop = AgentLoop::new(
            self.provider_registry.clone(),
            self.tool_registry.clone(),
            self.config.default_loop_config.clone(),
        )
        .with_transcript(transcript);

        let result = agent_loop.run(agent.as_ref(), ctx, message).await;

        // Record agent response messages to history
        if let Ok(ref messages) = result {
            for msg in messages {
                self.history_manager.push(session_id, msg.clone());
            }
        }

        // Remove from running
        self.running.remove(session_id);

        result
    }

    /// Abort a running agent execution.
    pub fn abort(&self, session_id: &str) -> bool {
        if let Some((_, handle)) = self.running.remove(session_id) {
            handle.abort_signal.abort();
            info!("Aborted agent execution: {}", session_id);
            true
        } else {
            warn!("No running agent found: {}", session_id);
            false
        }
    }

    /// Check if an agent is running.
    pub fn is_running(&self, session_id: &str) -> bool {
        self.running.contains_key(session_id)
    }

    /// Get the number of currently running agents.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Get session manager.
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Clear conversation history for a session.
    pub fn clear_history(&self, session_id: &str) {
        self.history_manager.clear(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::agent::AgentResponse;
    use std::collections::HashMap;

    struct MockAgent {
        config: AgentConfig,
    }

    impl MockAgent {
        fn new(id: &str) -> Self {
            Self {
                config: AgentConfig::new(id, "Mock Agent", "mock-model"),
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
                is_complete: true,
                tool_calls: Vec::new(),
                metadata: HashMap::new(),
            })
        }
    }

    #[test]
    fn test_runtime_config_default() {
        let config = AgentRuntimeConfig::default();
        assert_eq!(config.max_concurrent, 10);
    }

    #[test]
    fn test_runtime_creation() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentRuntimeConfig::default();

        let runtime = AgentRuntime::new(provider_registry, tool_registry, config);
        assert!(runtime.list_agents().is_empty());
    }

    #[test]
    fn test_register_agent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let agent = Arc::new(MockAgent::new("test-agent"));
        runtime.register_agent(agent);

        assert_eq!(runtime.list_agents().len(), 1);
        assert!(runtime.get_agent("test-agent").is_some());
    }

    #[test]
    fn test_unregister_agent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let agent = Arc::new(MockAgent::new("test-agent"));
        runtime.register_agent(agent);
        runtime.unregister_agent("test-agent");

        assert!(runtime.list_agents().is_empty());
    }

    #[tokio::test]
    async fn test_execute_agent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let agent = Arc::new(MockAgent::new("test-agent"));
        runtime.register_agent(agent);

        let message = Message::user("Hello");
        let result = runtime.execute("test-agent", "session-1", message).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_execute_nonexistent_agent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let message = Message::user("Hello");
        let result = runtime.execute("nonexistent", "session-1", message).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_running_count() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        assert_eq!(runtime.running_count(), 0);
        assert!(!runtime.is_running("session-1"));
    }

    #[test]
    fn test_abort_nonexistent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        assert!(!runtime.abort("nonexistent"));
    }

    #[test]
    fn test_runtime_config_debug() {
        let config = AgentRuntimeConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("AgentRuntimeConfig"));
    }

    #[test]
    fn test_runtime_config_clone() {
        let config = AgentRuntimeConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_concurrent, config.max_concurrent);
    }

    #[test]
    fn test_runtime_config_custom() {
        let config = AgentRuntimeConfig {
            max_concurrent: 5,
            default_loop_config: AgentLoopConfig {
                max_turns: 10,
                timeout_seconds: 60,
                checkpoint_enabled: false,
            },
        };
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.default_loop_config.max_turns, 10);
    }

    #[test]
    fn test_get_nonexistent_agent() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        assert!(runtime.get_agent("nonexistent").is_none());
    }

    #[test]
    fn test_session_manager_access() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let sm = runtime.session_manager();
        // Should be able to access session manager and it starts empty
        assert!(sm.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_agents_multiple() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        runtime.register_agent(Arc::new(MockAgent::new("agent1")));
        runtime.register_agent(Arc::new(MockAgent::new("agent2")));
        runtime.register_agent(Arc::new(MockAgent::new("agent3")));

        assert_eq!(runtime.list_agents().len(), 3);
    }

    #[test]
    fn test_agent_handle_fields() {
        let abort_signal = Arc::new(AbortSignal::new());
        let handle = AgentHandle {
            session_id: "test-session".to_string(),
            abort_signal: abort_signal.clone(),
        };
        assert_eq!(handle.session_id, "test-session");
        assert!(!handle.abort_signal.is_aborted());
    }

    #[test]
    fn test_history_manager_access() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let hm = runtime.history_manager();
        // Should be able to access history manager and it starts empty
        assert!(hm.get("nonexistent").is_empty());
    }

    #[test]
    fn test_clear_history() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        // Manually add some history
        runtime.history_manager().push("session-1", Message::user("Hello"));
        runtime.history_manager().push("session-1", Message::assistant("Hi"));
        assert_eq!(runtime.history_manager().get("session-1").len(), 2);

        // Clear and verify
        runtime.clear_history("session-1");
        assert!(runtime.history_manager().get("session-1").is_empty());
    }

    #[tokio::test]
    async fn test_execute_records_history() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

        let agent = Arc::new(MockAgent::new("test-agent"));
        runtime.register_agent(agent);

        // First message
        let message = Message::user("Hello");
        let result = runtime.execute("test-agent", "session-1", message).await;
        assert!(result.is_ok());

        // History should contain the user message and agent response
        let history = runtime.history_manager().get("session-1");
        assert!(history.len() >= 2); // At least user message + agent response
    }
}
