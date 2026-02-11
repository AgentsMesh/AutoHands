//! SingleTurnExecutor implementation methods.
//!
//! **Design:** SingleTurnExecutor is responsible for a single LLM call only.
//! It does NOT execute tools — tool execution is the sole responsibility of
//! `AgentLoop` in autohands-runtime, which prevents the double-execution bug
//! that occurred when both layers executed the same tool calls.

use tracing::{info, warn};

use autohands_protocols::agent::AgentResponse;
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::{CompletionRequest, CompletionResponse};
use autohands_protocols::types::{Message, StopReason};

use crate::executor::{SingleTurnExecutor, SingleTurnResult};

impl SingleTurnExecutor {
    /// Execute a single turn: one LLM call, returning tool_calls for the caller to execute.
    ///
    /// This method:
    /// 1. Builds a completion request from messages
    /// 2. Calls the LLM provider
    /// 3. Returns the result (including any tool_calls) for the caller to handle
    ///
    /// **Important:** This method does NOT execute tools. Tool execution is
    /// handled by `AgentLoop` to maintain a single point of control.
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
        let usage = response.usage.clone();
        match response.stop_reason {
            StopReason::EndTurn | StopReason::StopSequence => {
                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls: Vec::new(),
                    is_complete: true,
                    _stop_reason: response.stop_reason,
                    usage,
                })
            }
            StopReason::MaxTokens => {
                warn!("Max tokens reached in single turn");
                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls: Vec::new(),
                    is_complete: false,
                    _stop_reason: response.stop_reason,
                    usage,
                })
            }
            StopReason::ToolUse => {
                let tool_calls = response.message.tool_calls.clone();

                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls,
                    is_complete: false,
                    _stop_reason: response.stop_reason,
                    usage,
                })
            }
        }
    }

    /// Execute a complete interaction (for backward compatibility with Agent trait).
    ///
    /// This wraps `execute_turn` to provide a simple interface that returns
    /// an `AgentResponse`. The caller (typically `AgentLoop`) handles the
    /// loop control and tool execution.
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
            usage: Some(result.usage),
        })
    }

    /// Call the LLM provider.
    pub(crate) async fn call_llm(&self, request: CompletionRequest) -> Result<CompletionResponse, AgentError> {
        self.provider.complete(request).await.map_err(AgentError::from)
    }

    /// Record assistant message to transcript.
    pub(crate) async fn record_assistant_message(&self, response: &CompletionResponse) {
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

    pub(crate) fn build_request(&self, messages: &[Message]) -> CompletionRequest {
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
}
