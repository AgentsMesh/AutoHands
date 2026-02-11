//! Single-turn agent executor.
//!
//! This module provides a single-turn executor that handles one LLM call
//! and returns tool_calls for the caller to execute. Tool execution and
//! agentic loop control are handled by `AgentLoop` in autohands-runtime.

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
///
/// Contains the LLM response and any tool_calls requested by the model.
/// Tool execution is NOT performed here — it is the caller's responsibility
/// (typically `AgentLoop`) to execute the tools and feed results back.
#[derive(Debug)]
pub struct SingleTurnResult {
    /// The assistant's response message.
    pub message: Message,

    /// Tool calls requested by the LLM in this turn (if any).
    /// The caller is responsible for executing these tools.
    pub tool_calls: Vec<ToolCall>,

    /// Whether the agent considers the task complete.
    pub is_complete: bool,

    /// The stop reason from the LLM.
    pub _stop_reason: StopReason,

    /// Token usage for this turn.
    pub usage: autohands_protocols::types::Usage,
}

/// Single-turn executor for agent interactions.
///
/// This executor handles a single LLM call and returns the result
/// (including any tool_calls) for the caller to handle. It does NOT
/// execute tools or implement the agentic loop — those are the
/// responsibility of `AgentLoop` in autohands-runtime.
///
/// # Design Rationale (SRP + DRY)
///
/// - `SingleTurnExecutor`: Single LLM call only (request building + provider call)
/// - `AgentLoop`: Loop control, tool execution, abort handling, checkpointing
///
/// Tool execution is intentionally excluded from this layer to prevent
/// double-execution bugs and maintain a single point of control.
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
