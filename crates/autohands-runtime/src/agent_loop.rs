//! Agentic loop implementation.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::{debug, info, warn};

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::{Agent, AgentContext};
use autohands_protocols::error::AgentError;
use autohands_protocols::memory::{MemoryBackend, MemoryQuery};
use autohands_protocols::tool::ToolContext;
use autohands_protocols::types::Message;

use crate::checkpoint::CheckpointSupport;
use crate::memory_persistence;
use crate::summarizer::HistoryCompressor;
use crate::transcript::TranscriptWriter;

/// Configuration for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Enable checkpoint support.
    pub checkpoint_enabled: bool,
    /// 工具输出最大字符数，超出则截断并附加提示。0 表示不限制。
    pub max_tool_output_chars: usize,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            checkpoint_enabled: false,
            max_tool_output_chars: 100_000, // ~25K tokens
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
    tool_registry: Arc<ToolRegistry>,
    config: AgentLoopConfig,
    checkpoint: Option<Arc<dyn CheckpointSupport>>,
    transcript: Option<Arc<TranscriptWriter>>,
    compressor: Option<Arc<HistoryCompressor>>,
    memory_backend: Option<Arc<dyn MemoryBackend>>,
}

impl AgentLoop {
    pub fn new(
        _provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        config: AgentLoopConfig,
    ) -> Self {
        Self {
            tool_registry,
            config,
            checkpoint: None,
            transcript: None,
            compressor: None,
            memory_backend: None,
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

    /// Set history compressor for context length recovery.
    pub fn with_compressor(mut self, compressor: Arc<HistoryCompressor>) -> Self {
        self.compressor = Some(compressor);
        self
    }

    /// Set memory backend for context injection and flush.
    pub fn with_memory(mut self, backend: Arc<dyn MemoryBackend>) -> Self {
        self.memory_backend = Some(backend);
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

        // Memory context injection: search for memories related to the initial message
        self.inject_memory_context(&initial_message.content.text(), &mut messages)
            .await;

        messages.push(initial_message.clone());

        // Record initial user message to transcript
        if let Some(ref transcript) = self.transcript {
            let content = serde_json::to_value(&initial_message.content).unwrap_or_default();
            if let Err(e) = transcript.record_user_message(content).await {
                warn!("Failed to record user message to transcript: {}", e);
            }
        }

        self.run_loop_inner(agent, &mut ctx, messages, 0, &start_time).await
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

    /// Run from a specific turn with existing messages (checkpoint recovery).
    ///
    /// Fully aligned with `run()`: memory injection, transcript recording,
    /// context overflow recovery, usage accumulation, session end recording,
    /// and memory flush/session summary on exit.
    async fn run_from_turn(
        &self,
        agent: &dyn Agent,
        mut ctx: AgentContext,
        mut messages: Vec<Message>,
        start_turn: u32,
    ) -> Result<Vec<Message>, AgentError> {
        let start_time = std::time::Instant::now();

        // Memory context injection: use the most recent User message as search query
        // (checkpoint recovery has no new initial_message)
        let latest_user_text = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, autohands_protocols::types::MessageRole::User))
            .map(|m| m.content.text());

        if let Some(ref text) = latest_user_text {
            if !text.is_empty() {
                // Insert at position 0 so provider sees it before restored messages
                self.inject_memory_context_at(text, &mut messages, 0).await;
            }
        }

        self.run_loop_inner(agent, &mut ctx, messages, start_turn, &start_time).await
    }

    /// Inject memory context by appending a system message (used by `run()`).
    async fn inject_memory_context(&self, query_text: &str, messages: &mut Vec<Message>) {
        if let Some(ref memory) = self.memory_backend {
            let query = MemoryQuery {
                text: Some(query_text.to_string()),
                limit: 5,
                min_relevance: Some(0.3),
                ..Default::default()
            };
            match memory.search(query).await {
                Ok(results) if !results.is_empty() => {
                    let memory_ctx = memory_persistence::format_memory_context(&results);
                    messages.push(Message::system(&memory_ctx));
                    debug!(
                        "Injected {} relevant memories into context",
                        results.len()
                    );
                }
                Ok(_) => {
                    debug!("No relevant memories found for context injection");
                }
                Err(e) => {
                    warn!("Memory search for context injection failed: {}", e);
                }
            }
        }
    }

    /// Inject memory context at a specific position (used by `run_from_turn()`).
    async fn inject_memory_context_at(
        &self,
        query_text: &str,
        messages: &mut Vec<Message>,
        position: usize,
    ) {
        if let Some(ref memory) = self.memory_backend {
            let query = MemoryQuery {
                text: Some(query_text.to_string()),
                limit: 5,
                min_relevance: Some(0.3),
                ..Default::default()
            };
            match memory.search(query).await {
                Ok(results) if !results.is_empty() => {
                    let memory_ctx = memory_persistence::format_memory_context(&results);
                    messages.insert(position, Message::system(&memory_ctx));
                    debug!(
                        "Injected {} relevant memories into resumed context",
                        results.len()
                    );
                }
                Ok(_) => {
                    debug!("No relevant memories found for resumed context injection");
                }
                Err(e) => {
                    warn!("Memory search for resumed context injection failed: {}", e);
                }
            }
        }
    }

    /// Shared loop body for `run()` and `run_from_turn()`.
    ///
    /// Handles: turn iteration, abort/max-turns checks, agent processing with
    /// context-length recovery, transcript recording, token accumulation,
    /// checkpointing, completion/memory flush, and tool execution.
    async fn run_loop_inner(
        &self,
        agent: &dyn Agent,
        ctx: &mut AgentContext,
        mut messages: Vec<Message>,
        start_turn: u32,
        start_time: &std::time::Instant,
    ) -> Result<Vec<Message>, AgentError> {
        let mut turn = start_turn;
        let mut total_usage = autohands_protocols::types::Usage::default();

        loop {
            if ctx.abort_signal.is_aborted() {
                self.record_session_end("aborted", Some("User aborted"), turn, start_time)
                    .await;
                return Err(AgentError::Aborted);
            }

            turn += 1;
            debug!("Agent loop turn {}", turn);

            // Process through agent (with context length recovery)
            ctx.history = messages.clone();
            let last_msg = messages
                .last()
                .ok_or_else(|| AgentError::ExecutionFailed("Message history is empty".to_string()))?
                .clone();
            let response = match agent
                .process(last_msg, ctx.clone())
                .await
            {
                Ok(resp) => resp,
                Err(AgentError::ProviderError(ref provider_err))
                    if provider_err.is_context_length_error() =>
                {
                    warn!(
                        "Context length exceeded at turn {}, attempting compression",
                        turn
                    );
                    messages = self.compress_messages(messages).await?;
                    ctx.history = messages.clone();
                    let last_msg = messages
                        .last()
                        .ok_or_else(|| AgentError::ExecutionFailed("Message history is empty after compression".to_string()))?
                        .clone();
                    agent
                        .process(last_msg, ctx.clone())
                        .await?
                }
                Err(e) => return Err(e),
            };

            // Record assistant message to transcript
            if let Some(ref transcript) = self.transcript {
                let content =
                    serde_json::to_value(&response.message.content).unwrap_or_default();
                if let Err(e) = transcript.record_assistant_message(content, None).await {
                    warn!("Failed to record assistant message to transcript: {}", e);
                }
            }

            // Accumulate token usage
            if let Some(ref usage) = response.usage {
                total_usage.prompt_tokens += usage.prompt_tokens;
                total_usage.completion_tokens += usage.completion_tokens;
                total_usage.total_tokens += usage.total_tokens;
                debug!(
                    "Turn {} usage: prompt={}, completion={}, total={}; cumulative total={}",
                    turn,
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    usage.total_tokens,
                    total_usage.total_tokens
                );
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
                // Flush memory and store session summary on normal completion
                if let Some(ref memory) = self.memory_backend {
                    memory_persistence::flush_memories_to_backend(
                        &messages,
                        memory,
                        "session-end-flush",
                    )
                    .await;
                    memory_persistence::store_session_summary(
                        &messages,
                        &ctx.session_id,
                        memory,
                    )
                    .await;
                }
                self.record_session_end("completed", None, turn, start_time)
                    .await;
                break;
            }

            // Handle tool calls
            for tool_call in &response.tool_calls {
                // Record tool use to transcript
                if let Some(ref transcript) = self.transcript {
                    if let Err(e) = transcript
                        .record_tool_use(
                            &tool_call.id,
                            &tool_call.name,
                            tool_call.arguments.clone(),
                        )
                        .await
                    {
                        warn!("Failed to record tool use to transcript: {}", e);
                    }
                }

                let tool_start = std::time::Instant::now();
                let result = self.execute_tool(tool_call, ctx).await;
                let duration_ms = tool_start.elapsed().as_millis() as u64;

                // Record tool result to transcript
                if let Some(ref transcript) = self.transcript {
                    let is_error = result.starts_with("Error:");
                    if let Err(e) = transcript
                        .record_tool_result(
                            &tool_call.id,
                            &tool_call.name,
                            !is_error,
                            Some(&result),
                            if is_error { Some(&result) } else { None },
                            Some(duration_ms),
                        )
                        .await
                    {
                        warn!("Failed to record tool result to transcript: {}", e);
                    }
                }

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

        let work_dir = ctx
            .work_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let tool_ctx = ToolContext::new(&ctx.session_id, work_dir);

        let result = match tool.execute(tool_call.arguments.clone(), tool_ctx).await {
            Ok(result) => result.content,
            Err(e) => format!("Tool error: {}", e),
        };

        self.truncate_output(result)
    }

    /// 压缩消息历史，用于上下文长度恢复。
    async fn compress_messages(
        &self,
        messages: Vec<Message>,
    ) -> Result<Vec<Message>, AgentError> {
        // Memory flush: extract key information before compression
        if let Some(ref memory) = self.memory_backend {
            memory_persistence::flush_memories_to_backend(&messages, memory, "auto-flush").await;
        }

        if let Some(ref compressor) = self.compressor {
            match compressor.compress(messages).await {
                Ok((compressed, summary)) => {
                    if let Some(ref s) = summary {
                        info!(
                            "History compressed: {} messages summarized",
                            s.message_count
                        );
                    }
                    Ok(compressed)
                }
                Err(e) => {
                    warn!(
                        "History compression failed: {}, falling back to truncation",
                        e
                    );
                    Err(AgentError::ExecutionFailed(
                        "Context too large and compression failed".to_string(),
                    ))
                }
            }
        } else {
            // 无压缩器时的简单截断：丢弃前半部分消息
            warn!("No compressor available, truncating history by half");
            let len = messages.len();
            let keep = (len / 2).max(1);
            Ok(messages.into_iter().skip(len - keep).collect())
        }
    }

    /// 截断过大的工具输出，防止撑爆上下文。
    fn truncate_output(&self, content: String) -> String {
        let max = self.config.max_tool_output_chars;
        if max == 0 || content.len() <= max {
            return content;
        }
        warn!(
            "Tool output truncated: {} chars -> {} chars",
            content.len(),
            max
        );
        // 确保在 char 边界上截断
        let boundary = memory_persistence::floor_char_boundary(&content, max);
        let truncated = &content[..boundary];
        format!(
            "{}\n\n[OUTPUT TRUNCATED: original {} chars, showing first {}. Use more specific queries to reduce output size.]",
            truncated,
            content.len(),
            max
        )
    }
}

#[cfg(test)]
#[path = "agent_loop_tests.rs"]
mod tests;
