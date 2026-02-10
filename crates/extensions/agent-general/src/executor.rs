//! Single-turn agent executor.
//!
//! This module provides a single-turn executor that handles one LLM call
//! and optional tool execution. The agentic loop control is handled by
//! `AgentLoop` in autohands-runtime.

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;

use std::sync::Arc;

use autohands_protocols::agent::AgentConfig;
use autohands_protocols::provider::LLMProvider;
use autohands_protocols::tool::Tool;
use autohands_protocols::types::{Message, StopReason, ToolCall};
use autohands_runtime::TranscriptWriter;

/// Result of a single-turn execution.
#[derive(Debug)]
pub struct SingleTurnResult {
    /// The assistant's response message.
    pub message: Message,

    /// Tool calls made in this turn (if any).
    pub tool_calls: Vec<ToolCall>,

    /// Tool results (if tools were called).
    pub _tool_results: Vec<Message>,

    /// Whether the agent considers the task complete.
    pub is_complete: bool,

    /// The stop reason from the LLM.
    pub _stop_reason: StopReason,

    /// Token usage for this turn.
    pub usage: autohands_protocols::types::Usage,
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
    pub(crate) config: AgentConfig,
    pub(crate) provider: Arc<dyn LLMProvider>,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
    pub(crate) transcript: Option<Arc<TranscriptWriter>>,
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
}
