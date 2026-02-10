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
#[path = "agent_tests.rs"]
mod tests;
