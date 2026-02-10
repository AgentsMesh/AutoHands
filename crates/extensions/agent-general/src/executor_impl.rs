//! SingleTurnExecutor implementation methods.

use std::time::Instant;

use tracing::{debug, info, warn};

use autohands_protocols::agent::AgentResponse;
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::{CompletionRequest, CompletionResponse};
use autohands_protocols::tool::ToolContext;
use autohands_protocols::types::{Message, MessageContent, MessageRole, StopReason, ToolCall};

use crate::executor::{SingleTurnExecutor, SingleTurnResult};

impl SingleTurnExecutor {
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
        let usage = response.usage.clone();
        match response.stop_reason {
            StopReason::EndTurn | StopReason::StopSequence => {
                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls: Vec::new(),
                    _tool_results: Vec::new(),
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
                    _tool_results: Vec::new(),
                    is_complete: false,
                    _stop_reason: response.stop_reason,
                    usage,
                })
            }
            StopReason::ToolUse => {
                let tool_calls = response.message.tool_calls.clone();
                let tool_results = self.execute_tools(&tool_calls).await?;

                Ok(SingleTurnResult {
                    message: response.message,
                    tool_calls,
                    _tool_results: tool_results,
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

    pub(crate) async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Result<Vec<Message>, AgentError> {
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
                    let mut content = r.content;
                    let max = self.config.max_tool_output_chars;
                    if max > 0 && content.len() > max {
                        let original_len = content.len();
                        warn!(
                            "Tool {} output truncated: {} -> {} chars",
                            call.name, original_len, max
                        );
                        // Truncate at char boundary
                        let boundary = {
                            let mut i = max;
                            while i > 0 && !content.is_char_boundary(i) {
                                i -= 1;
                            }
                            i
                        };
                        content = format!(
                            "{}\n\n[OUTPUT TRUNCATED: {} chars -> {}]",
                            &content[..boundary],
                            original_len,
                            max
                        );
                    }
                    info!("Tool {} succeeded: {} chars", call.name, content.len());
                    (content, true, None)
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
