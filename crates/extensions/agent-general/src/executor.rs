//! Single-turn agent executor.
//!
//! This module provides a single-turn executor that handles one LLM call
//! and optional tool execution. The agentic loop control is handled by
//! `AgentLoop` in autohands-runtime.

use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, info, warn};

use autohands_protocols::agent::{AgentConfig, AgentResponse};
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::{CompletionRequest, CompletionResponse, LLMProvider};
use autohands_protocols::tool::{Tool, ToolContext};
use autohands_protocols::types::{Message, MessageContent, MessageRole, StopReason, ToolCall};
use autohands_runtime::TranscriptWriter;

/// Result of a single-turn execution.
#[derive(Debug)]
pub struct SingleTurnResult {
    /// The assistant's response message.
    pub message: Message,

    /// Tool calls made in this turn (if any).
    pub tool_calls: Vec<ToolCall>,

    /// Tool results (if tools were called).
    #[allow(dead_code)]
    pub tool_results: Vec<Message>,

    /// Whether the agent considers the task complete.
    pub is_complete: bool,

    /// The stop reason from the LLM.
    #[allow(dead_code)]
    pub stop_reason: StopReason,
}

/// Single-turn executor for agent interactions.
///
/// This executor handles a single LLM call and optional tool execution.
/// It does NOT implement the agentic loop - that's the responsibility
/// of `AgentLoop` in autohands-runtime.
///
/// # Design Rationale (DRY Principle)
///
/// Previously, both `AgentExecutor` and `AgentLoop` implemented similar
/// loop logic (abort checking, max_turns, tool execution). This violated
/// DRY and caused maintenance issues. Now:
///
/// - `SingleTurnExecutor`: Single LLM call + tool execution
/// - `AgentLoop`: Loop control, abort handling, checkpointing
pub struct SingleTurnExecutor {
    config: AgentConfig,
    provider: Arc<dyn LLMProvider>,
    tools: Vec<Arc<dyn Tool>>,
    transcript: Option<Arc<TranscriptWriter>>,
}

impl SingleTurnExecutor {
    /// Create a new single-turn executor.
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn LLMProvider>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            config,
            provider,
            tools,
            transcript: None,
        }
    }

    /// Set transcript writer for session recording.
    #[allow(dead_code)]
    pub fn with_transcript(mut self, transcript: Arc<TranscriptWriter>) -> Self {
        self.transcript = Some(transcript);
        self
    }

    /// Execute a single turn: one LLM call + optional tool execution.
    ///
    /// This method:
    /// 1. Builds a completion request from messages
    /// 2. Calls the LLM provider
    /// 3. If the LLM requests tools, executes them
    /// 4. Returns the result for the caller to decide next steps
    pub async fn execute_turn(
        &self,
        messages: &[Message],
    ) -> Result<SingleTurnResult, AgentError> {
        // Build completion request
        let request = self.build_request(messages);
        info!(
            "SingleTurnExecutor: {} tools, {} messages",
            request.tools.len(),
            request.messages.len()
        );

        // Get completion from LLM
        let response = self.call_llm(request).await?;

        // Record assistant message to transcript
        self.record_assistant_message(&response).await;

        // Process based on stop reason
        match response.stop_reason {
            StopReason::EndTurn | StopReason::StopSequence => {
                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls: Vec::new(),
                    tool_results: Vec::new(),
                    is_complete: true,
                    stop_reason: response.stop_reason,
                })
            }
            StopReason::MaxTokens => {
                warn!("Max tokens reached in single turn");
                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls: Vec::new(),
                    tool_results: Vec::new(),
                    is_complete: false,
                    stop_reason: response.stop_reason,
                })
            }
            StopReason::ToolUse => {
                let tool_calls = response.message.tool_calls.clone();
                let tool_results = self.execute_tools(&tool_calls).await?;

                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls,
                    tool_results,
                    is_complete: false,
                    stop_reason: response.stop_reason,
                })
            }
        }
    }

    /// Execute a complete interaction (for backward compatibility with Agent trait).
    ///
    /// This wraps `execute_turn` to provide a simple interface that returns
    /// an `AgentResponse`. The caller (typically `AgentLoop`) handles the
    /// loop control.
    pub async fn execute(
        &self,
        message: Message,
        history: Vec<Message>,
    ) -> Result<AgentResponse, AgentError> {
        let mut messages = history;
        messages.push(message);

        let result = self.execute_turn(&messages).await?;

        Ok(AgentResponse {
            message: result.message,
            is_complete: result.is_complete,
            tool_calls: result.tool_calls,
            metadata: Default::default(),
        })
    }

    /// Call the LLM provider.
    async fn call_llm(&self, request: CompletionRequest) -> Result<CompletionResponse, AgentError> {
        self.provider.complete(request).await.map_err(|e| {
            AgentError::ExecutionFailed(format!("Provider error: {}", e))
        })
    }

    /// Record assistant message to transcript.
    async fn record_assistant_message(&self, response: &CompletionResponse) {
        if let Some(ref transcript) = self.transcript {
            let content = serde_json::to_value(&response.message.content).unwrap_or_default();
            let stop_reason = format!("{:?}", response.stop_reason);
            if let Err(e) = transcript
                .record_assistant_message(content, Some(&stop_reason))
                .await
            {
                warn!("Failed to record assistant message: {}", e);
            }
        }
    }

    fn build_request(&self, messages: &[Message]) -> CompletionRequest {
        let tool_definitions: Vec<_> = self
            .tools
            .iter()
            .map(|t| t.definition().clone())
            .collect();

        let mut request =
            CompletionRequest::new(self.config.default_model.clone(), messages.to_vec())
                .with_tools(tool_definitions);

        if let Some(ref system) = self.config.system_prompt {
            request = request.with_system(system.clone());
        }

        request
    }

    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Result<Vec<Message>, AgentError> {
        let mut results = Vec::new();

        for call in tool_calls {
            debug!("Executing tool: {} ({})", call.name, call.id);

            // Record tool use to transcript
            if let Some(ref transcript) = self.transcript {
                if let Err(e) = transcript
                    .record_tool_use(&call.id, &call.name, call.arguments.clone())
                    .await
                {
                    warn!("Failed to record tool use: {}", e);
                }
            }

            let tool = self
                .tools
                .iter()
                .find(|t| t.definition().id == call.name)
                .ok_or_else(|| AgentError::NotFound(format!("Tool not found: {}", call.name)))?;

            let ctx = ToolContext::new(&call.id, std::env::current_dir().unwrap_or_default());

            let start = Instant::now();
            let result = tool.execute(call.arguments.clone(), ctx).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let (content, success, error_msg) = match result {
                Ok(r) => {
                    info!("Tool {} succeeded: {} chars", call.name, r.content.len());
                    (r.content, true, None)
                }
                Err(e) => {
                    warn!("Tool {} failed: {}", call.name, e);
                    let err = format!("Error: {}", e);
                    (err.clone(), false, Some(err))
                }
            };

            // Record tool result to transcript
            if let Some(ref transcript) = self.transcript {
                if let Err(e) = transcript
                    .record_tool_result(
                        &call.id,
                        &call.name,
                        success,
                        Some(&content),
                        error_msg.as_deref(),
                        Some(duration_ms),
                    )
                    .await
                {
                    warn!("Failed to record tool result: {}", e);
                }
            }

            results.push(Message {
                role: MessageRole::Tool,
                content: MessageContent::Text(content),
                name: None,
                tool_calls: Vec::new(),
                tool_call_id: Some(call.id.clone()),
                metadata: Default::default(),
            });
        }

        Ok(results)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::agent::AgentConfig;
    use autohands_protocols::error::ProviderError;
    use autohands_protocols::provider::{
        CompletionResponse, CompletionStream, ModelDefinition, ProviderCapabilities,
    };
    use autohands_protocols::tool::{ToolDefinition, ToolResult};
    use autohands_protocols::types::Usage;
    use autohands_protocols::error::ToolError;
    use std::collections::HashMap;

    struct MockProvider {
        response: CompletionResponse,
    }

    impl MockProvider {
        fn new(stop_reason: StopReason) -> Self {
            Self {
                response: CompletionResponse {
                    id: "test-response".to_string(),
                    model: "mock-model".to_string(),
                    message: Message::assistant("Hello!"),
                    stop_reason,
                    usage: Usage::default(),
                    metadata: HashMap::new(),
                },
            }
        }

        fn with_tool_call(call: ToolCall) -> Self {
            let mut msg = Message::assistant("I'll use a tool");
            msg.tool_calls = vec![call];
            Self {
                response: CompletionResponse {
                    id: "test-response".to_string(),
                    model: "mock-model".to_string(),
                    message: msg,
                    stop_reason: StopReason::ToolUse,
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

    struct MockTool {
        definition: ToolDefinition,
        result: String,
    }

    impl MockTool {
        fn new(name: &str, result: &str) -> Self {
            Self {
                definition: ToolDefinition::new(name, name, "A mock tool"),
                result: result.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success(&self.result))
        }
    }

    // Tests for SingleTurnExecutor

    #[test]
    fn test_executor_creation() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        assert_eq!(executor.config.id, "test");
    }

    #[test]
    fn test_executor_with_tools() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1", "result1")),
            Arc::new(MockTool::new("tool2", "result2")),
        ];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        assert_eq!(executor.tools.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_turn_completes() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_ok());
        let turn_result = result.unwrap();
        assert!(turn_result.is_complete);
        assert!(turn_result.tool_calls.is_empty());
        assert!(matches!(turn_result.stop_reason, StopReason::EndTurn));
    }

    #[tokio::test]
    async fn test_execute_turn_max_tokens() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::MaxTokens));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_ok());
        let turn_result = result.unwrap();
        assert!(!turn_result.is_complete);
        assert!(matches!(turn_result.stop_reason, StopReason::MaxTokens));
    }

    #[tokio::test]
    async fn test_execute_turn_stop_sequence() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::StopSequence));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_ok());
        let turn_result = result.unwrap();
        assert!(turn_result.is_complete);
    }

    #[tokio::test]
    async fn test_execute_turn_with_tool_use() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::with_tool_call(ToolCall {
            id: "call_1".to_string(),
            name: "test_tool".to_string(),
            arguments: serde_json::json!({}),
        }));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool::new("test_tool", "tool result"))];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_ok());
        let turn_result = result.unwrap();
        assert!(!turn_result.is_complete); // Tool use means not complete yet
        assert_eq!(turn_result.tool_calls.len(), 1);
        assert_eq!(turn_result.tool_results.len(), 1);
        assert!(matches!(turn_result.stop_reason, StopReason::ToolUse));
    }

    #[tokio::test]
    async fn test_execute_backward_compat() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_complete);
    }

    #[tokio::test]
    async fn test_execute_with_history() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let message = Message::user("Follow-up");
        let history = vec![
            Message::user("Initial question"),
            Message::assistant("Initial answer"),
        ];
        let result = executor.execute(message, history).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_build_request_without_system() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let request = executor.build_request(&messages);

        assert_eq!(request.model, "mock-model");
        assert_eq!(request.messages.len(), 1);
        assert!(request.system.is_none());
    }

    #[test]
    fn test_build_request_with_system() {
        let mut config = AgentConfig::new("test", "Test Agent", "mock-model");
        config.system_prompt = Some("You are a helpful assistant".to_string());
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let request = executor.build_request(&messages);

        assert!(request.system.is_some());
        assert_eq!(request.system.unwrap(), "You are a helpful assistant");
    }

    #[test]
    fn test_build_request_with_tools() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool::new("read_file", "content"))];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let request = executor.build_request(&messages);

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].id, "read_file");
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        fn id(&self) -> &str {
            "failing"
        }
        fn models(&self) -> &[ModelDefinition] {
            &[]
        }
        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                streaming: false,
                tool_calling: false,
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
            Err(ProviderError::Network("Connection failed".to_string()))
        }
        async fn complete_stream(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[tokio::test]
    async fn test_execute_turn_provider_error() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(FailingProvider);
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::ExecutionFailed(msg) => assert!(msg.contains("Provider error")),
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_execute_turn_tool_not_found() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::with_tool_call(ToolCall {
            id: "call_1".to_string(),
            name: "nonexistent_tool".to_string(),
            arguments: serde_json::json!({}),
        }));
        let tools: Vec<Arc<dyn Tool>> = vec![]; // No tools registered

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::NotFound(msg) => assert!(msg.contains("nonexistent_tool")),
            _ => panic!("Expected NotFound error"),
        }
    }

    struct FailingTool {
        definition: ToolDefinition,
    }

    impl FailingTool {
        fn new(name: &str) -> Self {
            Self {
                definition: ToolDefinition::new(name, name, "A failing tool"),
            }
        }
    }

    #[async_trait]
    impl Tool for FailingTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Err(ToolError::ExecutionFailed("Tool failed".to_string()))
        }
    }

    #[tokio::test]
    async fn test_execute_turn_failing_tool() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::with_tool_call(ToolCall {
            id: "call_1".to_string(),
            name: "test_tool".to_string(),
            arguments: serde_json::json!({}),
        }));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(FailingTool::new("test_tool"))];

        let executor = SingleTurnExecutor::new(config, provider, tools);
        let messages = vec![Message::user("Hello")];
        let result = executor.execute_turn(&messages).await;

        // Even with failing tool, execution should continue
        // The error should be captured in the tool result message
        assert!(result.is_ok());
        let turn_result = result.unwrap();
        assert_eq!(turn_result.tool_results.len(), 1);
        // Tool result should contain error message
        assert!(turn_result.tool_results[0].content.text().contains("Error"));
    }

    // Tests for SingleTurnResult
    #[test]
    fn test_single_turn_result_debug() {
        let result = SingleTurnResult {
            message: Message::assistant("Test"),
            tool_calls: vec![],
            tool_results: vec![],
            is_complete: true,
            stop_reason: StopReason::EndTurn,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("SingleTurnResult"));
    }

    // Tests for AgentConfig (from protocols)
    #[test]
    fn test_executor_config() {
        let config = AgentConfig::new("test", "Test", "model");
        assert_eq!(config.max_turns, 50);
        assert_eq!(config.id, "test");
        assert_eq!(config.default_model, "model");
    }

    #[test]
    fn test_config_with_system_prompt() {
        let mut config = AgentConfig::new("test", "Test", "model");
        config.system_prompt = Some("You are helpful".to_string());
        assert!(config.system_prompt.is_some());
    }

    // Tests for types
    #[test]
    fn test_agent_response_complete() {
        let response = AgentResponse {
            message: Message::assistant("Done"),
            is_complete: true,
            tool_calls: vec![],
            metadata: Default::default(),
        };
        assert!(response.is_complete);
        assert!(response.tool_calls.is_empty());
    }

    #[test]
    fn test_tool_call_creation() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "search".to_string(),
            arguments: serde_json::json!({"query": "rust"}),
        };
        assert_eq!(call.id, "call_123");
        assert_eq!(call.name, "search");
        assert_eq!(call.arguments["query"], "rust");
    }

    #[test]
    fn test_message_role_variants() {
        let user = Message::user("Hello");
        assert!(matches!(user.role, MessageRole::User));

        let assistant = Message::assistant("Hi");
        assert!(matches!(assistant.role, MessageRole::Assistant));

        let tool = Message::tool("tool_id", "result".to_string());
        assert!(matches!(tool.role, MessageRole::Tool));
    }

    #[test]
    fn test_stop_reason_variants() {
        assert!(matches!(StopReason::EndTurn, StopReason::EndTurn));
        assert!(matches!(StopReason::MaxTokens, StopReason::MaxTokens));
        assert!(matches!(StopReason::ToolUse, StopReason::ToolUse));
        assert!(matches!(StopReason::StopSequence, StopReason::StopSequence));
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::Aborted;
        assert!(err.to_string().contains("borted"));

        let err = AgentError::MaxTurnsExceeded(50);
        assert!(err.to_string().contains("50"));

        let err = AgentError::NotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = AgentError::ExecutionFailed("reason".to_string());
        assert!(err.to_string().contains("reason"));
    }
}
