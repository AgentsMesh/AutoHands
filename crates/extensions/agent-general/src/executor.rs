//! Agent executor for the agentic loop.

use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, info, trace, warn};

use autohands_protocols::agent::{AgentConfig, AgentResponse};
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::{CompletionRequest, LLMProvider};
use autohands_protocols::tool::{AbortSignal, Tool, ToolContext};
use autohands_protocols::types::{Message, MessageContent, MessageRole, StopReason, ToolCall};
use autohands_runtime::TranscriptWriter;

/// Executor for the agentic loop.
pub struct AgentExecutor {
    config: AgentConfig,
    provider: Arc<dyn LLMProvider>,
    tools: Vec<Arc<dyn Tool>>,
    abort_signal: Arc<AbortSignal>,
    transcript: Option<Arc<TranscriptWriter>>,
}

impl AgentExecutor {
    /// Create a new executor.
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn LLMProvider>,
        tools: Vec<Arc<dyn Tool>>,
        abort_signal: Arc<AbortSignal>,
    ) -> Self {
        Self {
            config,
            provider,
            tools,
            abort_signal,
            transcript: None,
        }
    }

    /// Set transcript writer for session recording.
    pub fn with_transcript(mut self, transcript: Arc<TranscriptWriter>) -> Self {
        self.transcript = Some(transcript);
        self
    }

    /// Execute the agentic loop.
    pub async fn execute(
        &self,
        message: Message,
        history: Vec<Message>,
    ) -> Result<AgentResponse, AgentError> {
        let start_time = Instant::now();
        let mut messages = history;
        messages.push(message.clone());

        let mut turns = 0;
        let mut all_tool_calls = Vec::new();

        info!(
            "AgentExecutor starting with {} tools available",
            self.tools.len()
        );
        for tool in &self.tools {
            debug!("  - Tool: {}", tool.definition().id);
        }

        // Record user message to transcript
        if let Some(ref transcript) = self.transcript {
            let content = serde_json::to_value(&message.content).unwrap_or_default();
            if let Err(e) = transcript.record_user_message(content).await {
                warn!("Failed to record user message: {}", e);
            }
        }

        loop {
            // Check abort signal
            if self.abort_signal.is_aborted() {
                self.record_session_end("aborted", Some("User aborted"), turns, &start_time).await;
                return Err(AgentError::Aborted);
            }

            // Check max turns
            if turns >= self.config.max_turns {
                self.record_session_end("max_turns", Some("Max turns exceeded"), turns, &start_time).await;
                return Err(AgentError::MaxTurnsExceeded(self.config.max_turns));
            }

            turns += 1;
            debug!("Agent turn {}/{}", turns, self.config.max_turns);

            // Build completion request
            let request = self.build_request(&messages);
            info!(
                "Sending request to LLM with {} tools, {} messages",
                request.tools.len(),
                request.messages.len()
            );

            // Get completion
            let response = self.provider.complete(request).await
                .map_err(|e| {
                    let err_msg = format!("Provider error: {}", e);
                    // Record error to transcript (fire and forget)
                    let transcript = self.transcript.clone();
                    let turns_copy = turns;
                    let start_copy = start_time;
                    let err_msg_clone = err_msg.clone();
                    tokio::spawn(async move {
                        if let Some(t) = transcript {
                            let _ = t.record_session_end("error", Some(&err_msg_clone), turns_copy, Some(start_copy.elapsed().as_millis() as u64)).await;
                        }
                    });
                    AgentError::ExecutionFailed(err_msg)
                })?;

            // Record assistant message to transcript
            if let Some(ref transcript) = self.transcript {
                let content = serde_json::to_value(&response.message.content).unwrap_or_default();
                let stop_reason = format!("{:?}", response.stop_reason);
                if let Err(e) = transcript.record_assistant_message(content, Some(&stop_reason)).await {
                    warn!("Failed to record assistant message: {}", e);
                }
            }

            // Add assistant message to history
            messages.push(response.message.clone());

            // Check stop reason
            match response.stop_reason {
                StopReason::EndTurn | StopReason::StopSequence => {
                    info!("Agent completed in {} turns", turns);
                    self.record_session_end("completed", None, turns, &start_time).await;
                    return Ok(AgentResponse {
                        message: response.message,
                        is_complete: true,
                        tool_calls: all_tool_calls,
                        metadata: Default::default(),
                    });
                }
                StopReason::MaxTokens => {
                    warn!("Max tokens reached");
                    self.record_session_end("max_tokens", None, turns, &start_time).await;
                    return Ok(AgentResponse {
                        message: response.message,
                        is_complete: false,
                        tool_calls: all_tool_calls,
                        metadata: Default::default(),
                    });
                }
                StopReason::ToolUse => {
                    // Execute tool calls
                    let tool_results = self.execute_tools(&response.message.tool_calls).await?;
                    all_tool_calls.extend(response.message.tool_calls.clone());

                    // Add tool results to messages
                    for result in tool_results {
                        messages.push(result);
                    }
                }
            }
        }
    }

    /// Record session end to transcript.
    async fn record_session_end(&self, status: &str, error: Option<&str>, turns: u32, start_time: &Instant) {
        if let Some(ref transcript) = self.transcript {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            if let Err(e) = transcript.record_session_end(status, error, turns, Some(duration_ms)).await {
                warn!("Failed to record session end: {}", e);
            }
        }
    }

    fn build_request(&self, messages: &[Message]) -> CompletionRequest {
        let tool_definitions: Vec<_> = self.tools.iter()
            .map(|t| t.definition().clone())
            .collect();

        let mut request = CompletionRequest::new(
            self.config.default_model.clone(),
            messages.to_vec(),
        )
        .with_tools(tool_definitions);

        if let Some(ref system) = self.config.system_prompt {
            request = request.with_system(system.clone());
        }

        request
    }

    async fn execute_tools(
        &self,
        tool_calls: &[ToolCall],
    ) -> Result<Vec<Message>, AgentError> {
        let mut results = Vec::new();

        for call in tool_calls {
            debug!("Executing tool: {} ({})", call.name, call.id);

            // Record tool use to transcript
            if let Some(ref transcript) = self.transcript {
                if let Err(e) = transcript.record_tool_use(
                    &call.id,
                    &call.name,
                    call.arguments.clone(),
                ).await {
                    warn!("Failed to record tool use: {}", e);
                }
            }

            let tool = self.tools.iter()
                .find(|t| t.definition().id == call.name)
                .ok_or_else(|| AgentError::NotFound(format!("Tool not found: {}", call.name)))?;

            let ctx = ToolContext::new(
                &call.id,
                std::env::current_dir().unwrap_or_default(),
            );

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
                if let Err(e) = transcript.record_tool_result(
                    &call.id,
                    &call.name,
                    success,
                    Some(&content),
                    error_msg.as_deref(),
                    Some(duration_ms),
                ).await {
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
    use autohands_protocols::provider::{CompletionResponse, CompletionStream, ModelDefinition, ProviderCapabilities};
    use autohands_protocols::tool::{ToolDefinition, ToolResult};
    use autohands_protocols::error::{ProviderError, ToolError};
    use autohands_protocols::types::Usage;
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

        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            Ok(self.response.clone())
        }

        async fn complete_stream(&self, _req: CompletionRequest) -> Result<CompletionStream, ProviderError> {
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

    #[test]
    fn test_abort_signal() {
        let signal = AbortSignal::new();
        assert!(!signal.is_aborted());
        signal.abort();
        assert!(signal.is_aborted());
    }

    #[test]
    fn test_abort_signal_default() {
        let signal = AbortSignal::default();
        assert!(!signal.is_aborted());
    }

    #[test]
    fn test_abort_signal_multiple_aborts() {
        let signal = AbortSignal::new();
        signal.abort();
        signal.abort(); // Should be safe to call multiple times
        assert!(signal.is_aborted());
    }

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
    fn test_agent_response_with_tool_calls() {
        let tool_call = ToolCall {
            id: "tc_1".to_string(),
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        };
        let response = AgentResponse {
            message: Message::assistant("Let me read that file"),
            is_complete: false,
            tool_calls: vec![tool_call],
            metadata: Default::default(),
        };
        assert!(!response.is_complete);
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "read_file");
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

    #[test]
    fn test_executor_creation() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
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
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        assert_eq!(executor.tools.len(), 2);
    }

    #[tokio::test]
    async fn test_executor_execute_completes() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_complete);
    }

    #[tokio::test]
    async fn test_executor_execute_max_tokens() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::MaxTokens));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_complete); // MaxTokens = not complete
    }

    #[tokio::test]
    async fn test_executor_execute_aborted() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());
        abort_signal.abort(); // Abort before execution

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::Aborted));
    }

    #[tokio::test]
    async fn test_executor_execute_with_history() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
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
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
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
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let messages = vec![Message::user("Hello")];
        let request = executor.build_request(&messages);

        assert!(request.system.is_some());
        assert_eq!(request.system.unwrap(), "You are a helpful assistant");
    }

    #[test]
    fn test_build_request_with_tools() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("read_file", "content")),
        ];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let messages = vec![Message::user("Hello")];
        let request = executor.build_request(&messages);

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].id, "read_file");
    }

    #[tokio::test]
    async fn test_executor_execute_stop_sequence() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::StopSequence));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_complete); // StopSequence = complete
    }

    #[tokio::test]
    async fn test_executor_execute_max_turns_exceeded() {
        let mut config = AgentConfig::new("test", "Test Agent", "mock-model");
        config.max_turns = 0; // Set to 0 to trigger max turns immediately
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new(StopReason::EndTurn));
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::MaxTurnsExceeded(max) => assert_eq!(max, 0),
            _ => panic!("Expected MaxTurnsExceeded error"),
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        fn id(&self) -> &str { "failing" }
        fn models(&self) -> &[ModelDefinition] { &[] }
        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                streaming: false, tool_calling: false, vision: false,
                json_mode: false, prompt_caching: false, batching: false,
                max_concurrent: None,
            }
        }
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Network("Connection failed".to_string()))
        }
        async fn complete_stream(&self, _req: CompletionRequest) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[tokio::test]
    async fn test_executor_execute_provider_error() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(FailingProvider);
        let tools: Vec<Arc<dyn Tool>> = vec![];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::ExecutionFailed(msg) => assert!(msg.contains("Provider error")),
            _ => panic!("Expected ExecutionFailed error"),
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

    // Provider that returns tool use, then ends
    struct ToolUseProvider {
        call_count: std::sync::atomic::AtomicU32,
    }

    impl ToolUseProvider {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for ToolUseProvider {
        fn id(&self) -> &str { "tool_use" }
        fn models(&self) -> &[ModelDefinition] { &[] }
        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                streaming: false, tool_calling: true, vision: false,
                json_mode: false, prompt_caching: false, batching: false,
                max_concurrent: None,
            }
        }
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                // First call: return tool use
                let mut msg = Message::assistant("Using tool");
                msg.tool_calls = vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "test_tool".to_string(),
                    arguments: serde_json::json!({}),
                }];
                Ok(CompletionResponse {
                    id: "resp".to_string(),
                    model: "mock".to_string(),
                    message: msg,
                    stop_reason: StopReason::ToolUse,
                    usage: Usage::default(),
                    metadata: HashMap::new(),
                })
            } else {
                // Second call: end turn
                Ok(CompletionResponse {
                    id: "resp".to_string(),
                    model: "mock".to_string(),
                    message: Message::assistant("Done"),
                    stop_reason: StopReason::EndTurn,
                    usage: Usage::default(),
                    metadata: HashMap::new(),
                })
            }
        }
        async fn complete_stream(&self, _req: CompletionRequest) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[tokio::test]
    async fn test_executor_with_tool_use() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(ToolUseProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("test_tool", "tool result")),
        ];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_complete);
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_executor_with_tool_not_found() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        // Provider returns tool call for non-existent tool
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::with_tool_call(ToolCall {
            id: "call_1".to_string(),
            name: "nonexistent_tool".to_string(),
            arguments: serde_json::json!({}),
        }));
        let tools: Vec<Arc<dyn Tool>> = vec![]; // No tools registered
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::NotFound(msg) => assert!(msg.contains("nonexistent_tool")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_executor_with_failing_tool() {
        let config = AgentConfig::new("test", "Test Agent", "mock-model");
        let provider: Arc<dyn LLMProvider> = Arc::new(ToolUseProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(FailingTool::new("test_tool")),
        ];
        let abort_signal = Arc::new(AbortSignal::new());

        let executor = AgentExecutor::new(config, provider, tools, abort_signal);
        let message = Message::user("Hello");
        let result = executor.execute(message, vec![]).await;

        // Even with failing tool, execution should continue
        // The error should be captured in the tool result message
        assert!(result.is_ok());
    }
}
