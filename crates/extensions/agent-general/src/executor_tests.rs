//! Tests for SingleTurnExecutor.

use super::*;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use autohands_protocols::agent::{AgentConfig, AgentResponse};
use autohands_protocols::error::{AgentError, ProviderError, ToolError};
use autohands_protocols::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LLMProvider,
    ModelDefinition, ProviderCapabilities,
};
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::{MessageRole, Usage};

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
    assert!(matches!(turn_result._stop_reason, StopReason::EndTurn));
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
    assert!(matches!(turn_result._stop_reason, StopReason::MaxTokens));
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
    // SingleTurnExecutor does NOT execute tools — tool_calls are returned
    // for AgentLoop to execute, preventing double-execution.
    assert_eq!(turn_result.tool_calls[0].name, "test_tool");
    assert!(matches!(turn_result._stop_reason, StopReason::ToolUse));
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
        AgentError::ProviderError(_) => {}
        _ => panic!("Expected ProviderError"),
    }
}

#[tokio::test]
async fn test_execute_turn_tool_not_found_returns_tool_calls() {
    // SingleTurnExecutor does NOT execute tools, so even a nonexistent tool name
    // is simply returned as a tool_call for AgentLoop to handle.
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

    // No error — executor just returns tool_calls without executing
    assert!(result.is_ok());
    let turn_result = result.unwrap();
    assert_eq!(turn_result.tool_calls.len(), 1);
    assert_eq!(turn_result.tool_calls[0].name, "nonexistent_tool");
    assert!(!turn_result.is_complete);
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
async fn test_execute_turn_with_failing_tool_returns_tool_calls() {
    // SingleTurnExecutor does NOT execute tools, so even a "failing" tool
    // simply has its tool_call returned. Failure handling is AgentLoop's job.
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

    assert!(result.is_ok());
    let turn_result = result.unwrap();
    // Tool calls returned without execution
    assert_eq!(turn_result.tool_calls.len(), 1);
    assert_eq!(turn_result.tool_calls[0].name, "test_tool");
    assert!(!turn_result.is_complete);
}

// Tests for SingleTurnResult
#[test]
fn test_single_turn_result_debug() {
    let result = SingleTurnResult {
        message: Message::assistant("Test"),
        tool_calls: vec![],
        is_complete: true,
        _stop_reason: StopReason::EndTurn,
        usage: Usage::default(),
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
        usage: None,
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
