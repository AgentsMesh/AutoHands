//! General purpose agent implementation.
//!
//! This module provides a general-purpose agent that processes messages
//! using an LLM provider and can execute tools. The agent handles
//! single-turn interactions; the agentic loop is managed by `AgentLoop`.

use async_trait::async_trait;
use std::sync::Arc;

use autohands_protocols::agent::{Agent, AgentConfig, AgentContext, AgentResponse};
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::LLMProvider;
use autohands_protocols::tool::Tool;
use autohands_protocols::types::Message;

use crate::executor::SingleTurnExecutor;

/// General purpose agent that can use tools.
///
/// # Design
///
/// The `GeneralAgent` implements the `Agent` trait and handles single-turn
/// message processing. It uses `SingleTurnExecutor` internally for:
/// - Building LLM requests
/// - Calling the LLM provider
/// - Executing tool calls
///
/// The agentic loop (multiple turns, abort checking, max_turns enforcement)
/// is handled by `AgentLoop` in `autohands-runtime`, NOT by this agent.
/// This separation follows the Single Responsibility Principle.
pub struct GeneralAgent {
    config: AgentConfig,
    provider: Arc<dyn LLMProvider>,
    tools: Vec<Arc<dyn Tool>>,
}

impl GeneralAgent {
    /// Create a new general agent.
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn LLMProvider>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            config,
            provider,
            tools,
        }
    }

    /// Create a single-turn executor for this agent.
    fn executor(&self) -> SingleTurnExecutor {
        SingleTurnExecutor::new(
            self.config.clone(),
            self.provider.clone(),
            self.tools.clone(),
        )
    }
}

#[async_trait]
impl Agent for GeneralAgent {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Process a single message and return a response.
    ///
    /// This method handles one turn of interaction:
    /// 1. Calls the LLM with the message and history
    /// 2. If the LLM requests tools, executes them
    /// 3. Returns the response
    ///
    /// Note: This does NOT implement the full agentic loop. The caller
    /// (typically `AgentLoop`) is responsible for:
    /// - Checking abort signals
    /// - Enforcing max_turns limits
    /// - Continuing the conversation if not complete
    async fn process(
        &self,
        message: Message,
        ctx: AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let executor = self.executor();
        executor.execute(message, ctx.history).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::error::ProviderError;
    use autohands_protocols::provider::{
        CompletionRequest, CompletionResponse, CompletionStream, ModelDefinition,
        ProviderCapabilities,
    };
    use autohands_protocols::types::{StopReason, Usage};
    use std::collections::HashMap;

    struct MockProvider {
        response: CompletionResponse,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                response: CompletionResponse {
                    id: "test-response".to_string(),
                    model: "mock-model".to_string(),
                    message: Message::assistant("Test response"),
                    stop_reason: StopReason::EndTurn,
                    usage: Usage::default(),
                    metadata: HashMap::new(),
                },
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelDefinition] {
            &[]
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                streaming: false,
                tool_calling: true,
                vision: false,
                json_mode: false,
                prompt_caching: false,
                batching: false,
                max_concurrent: None,
            }
        }

        async fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, ProviderError> {
            Ok(self.response.clone())
        }

        async fn complete_stream(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new("test-agent", "Test Agent", "test-model");
        assert_eq!(config.id, "test-agent");
        assert_eq!(config.name, "Test Agent");
        assert_eq!(config.default_model, "test-model");
    }

    #[test]
    fn test_agent_config_max_turns() {
        let config = AgentConfig::new("test", "Test", "model");
        assert_eq!(config.max_turns, 50); // default value
    }

    #[test]
    fn test_agent_config_with_system_prompt() {
        let mut config = AgentConfig::new("test", "Test", "model");
        config.system_prompt = Some("You are a helpful assistant.".to_string());
        assert_eq!(
            config.system_prompt,
            Some("You are a helpful assistant.".to_string())
        );
    }

    #[test]
    fn test_general_agent_creation() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config, provider, tools);
        assert_eq!(agent.id(), "test-agent");
    }

    #[test]
    fn test_general_agent_config() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config.clone(), provider, tools);
        let agent_config = agent.config();
        assert_eq!(agent_config.id, "test-agent");
        assert_eq!(agent_config.name, "Test Agent");
        assert_eq!(agent_config.default_model, "mock-model");
    }

    #[test]
    fn test_general_agent_executor_creation() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config, provider, tools);
        let _executor = agent.executor();
        // Just verifies that executor can be created without panic
    }

    #[tokio::test]
    async fn test_general_agent_process() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config, provider, tools);
        let ctx = AgentContext::new("session-1");
        let message = Message::user("Hello");

        let result = agent.process(message, ctx).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_complete);
    }

    #[tokio::test]
    async fn test_general_agent_process_with_history() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config, provider, tools);
        let history = vec![
            Message::user("Previous message"),
            Message::assistant("Previous response"),
        ];
        let ctx = AgentContext::new("session-1").with_history(history);
        let message = Message::user("Follow-up");

        let result = agent.process(message, ctx).await;
        assert!(result.is_ok());
    }

    // Note: Abort signal checking is now handled by AgentLoop, not by GeneralAgent.
    // The agent's process() method handles a single turn and does not check abort signals.
    // See autohands-runtime/src/agent_loop.rs for abort handling tests.
}
