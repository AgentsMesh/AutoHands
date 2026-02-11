//! Agent protocol definitions.
//!
//! Agents are the core execution units that process user requests.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::AgentError;
use crate::types::{Message, Metadata};

/// Core trait for agents.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Returns the agent ID.
    fn id(&self) -> &str;

    /// Returns the agent configuration.
    fn config(&self) -> &AgentConfig;

    /// Process a user message and return the response.
    async fn process(
        &self,
        message: Message,
        ctx: AgentContext,
    ) -> Result<AgentResponse, AgentError>;

    /// Check if the agent can handle a given message.
    fn can_handle(&self, message: &Message) -> bool {
        let _ = message;
        true
    }
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent ID.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of the agent.
    pub description: String,

    /// Default model to use.
    pub default_model: String,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Maximum turns per conversation.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Tool IDs this agent can use.
    #[serde(default)]
    pub tools: Vec<String>,

    /// Skill IDs this agent can use.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,

    /// 工具输出最大字符数，超出则截断。0 表示不限制。
    #[serde(default = "default_max_tool_output_chars")]
    pub max_tool_output_chars: usize,
}

fn default_max_turns() -> u32 {
    50
}

fn default_timeout() -> u64 {
    300
}

fn default_max_tool_output_chars() -> usize {
    100_000
}

impl AgentConfig {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            default_model: default_model.into(),
            system_prompt: None,
            max_turns: default_max_turns(),
            timeout_seconds: default_timeout(),
            tools: Vec::new(),
            skills: Vec::new(),
            metadata: HashMap::new(),
            max_tool_output_chars: default_max_tool_output_chars(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }
}

/// Context for agent execution.
#[derive(Clone)]
pub struct AgentContext {
    /// Session ID.
    pub session_id: String,

    /// Conversation history.
    pub history: Vec<Message>,

    /// Abort signal for cancellation.
    pub abort_signal: std::sync::Arc<crate::tool::AbortSignal>,

    /// Additional context data.
    pub data: HashMap<String, serde_json::Value>,

    /// Working directory for tool execution. Falls back to `current_dir()` if `None`.
    pub work_dir: Option<PathBuf>,
}

impl AgentContext {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            history: Vec::new(),
            abort_signal: std::sync::Arc::new(crate::tool::AbortSignal::new()),
            data: HashMap::new(),
            work_dir: None,
        }
    }

    pub fn with_work_dir(mut self, work_dir: PathBuf) -> Self {
        self.work_dir = Some(work_dir);
        self
    }

    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        self.history = history;
        self
    }
}

/// Response from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// The response message.
    pub message: Message,

    /// Whether the conversation is complete.
    pub is_complete: bool,

    /// Tool calls that were made.
    #[serde(default)]
    pub tool_calls: Vec<crate::types::ToolCall>,

    /// Metadata about the response.
    #[serde(default)]
    pub metadata: Metadata,

    /// Token usage for this response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::types::Usage>,
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
