//! General purpose agent implementation.

use async_trait::async_trait;
use std::sync::Arc;

use autohands_protocols::agent::{Agent, AgentConfig, AgentContext, AgentResponse};
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::LLMProvider;
use autohands_protocols::tool::Tool;
use autohands_protocols::types::Message;

use crate::executor::AgentExecutor;

/// General purpose agent that can use tools.
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

    /// Create an executor for this agent.
    fn executor(&self, ctx: &AgentContext) -> AgentExecutor {
        AgentExecutor::new(
            self.config.clone(),
            self.provider.clone(),
            self.tools.clone(),
            ctx.abort_signal.clone(),
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

    async fn process(
        &self,
        message: Message,
        ctx: AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let executor = self.executor(&ctx);
        executor.execute(message, ctx.history).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::provider::{CompletionRequest, CompletionResponse, CompletionStream, ModelDefinition, ProviderCapabilities};
    use autohands_protocols::error::ProviderError;
    use autohands_protocols::types::{StopReason, Usage};
    use autohands_protocols::tool::AbortSignal;
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

        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            Ok(self.response.clone())
        }

        async fn complete_stream(&self, _req: CompletionRequest) -> Result<CompletionStream, ProviderError> {
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
        assert_eq!(config.system_prompt, Some("You are a helpful assistant.".to_string()));
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
        let ctx = AgentContext::new("session-1");
        let _executor = agent.executor(&ctx);
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

    #[tokio::test]
    async fn test_general_agent_aborted() {
        let config = AgentConfig::new("test-agent", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = GeneralAgent::new(config, provider, tools);
        let abort_signal = Arc::new(AbortSignal::new());
        abort_signal.abort();
        let ctx = AgentContext {
            session_id: "session-1".to_string(),
            history: Vec::new(),
            abort_signal,
            data: HashMap::new(),
        };
        let message = Message::user("Hello");

        let result = agent.process(message, ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::Aborted));
    }
}
