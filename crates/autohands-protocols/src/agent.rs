//! Agent protocol definitions.
//!
//! Agents are the core execution units that process user requests.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

impl AgentContext {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            history: Vec::new(),
            abort_signal: std::sync::Arc::new(crate::tool::AbortSignal::new()),
            data: HashMap::new(),
        }
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
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_new() {
        let config = AgentConfig::new("test-agent", "Test Agent", "gpt-4");
        assert_eq!(config.id, "test-agent");
        assert_eq!(config.name, "Test Agent");
        assert_eq!(config.default_model, "gpt-4");
        assert!(config.description.is_empty());
        assert!(config.system_prompt.is_none());
        assert_eq!(config.max_turns, 50);
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_agent_config_with_system_prompt() {
        let config = AgentConfig::new("test", "Test", "gpt-4")
            .with_system_prompt("You are a helpful assistant.");
        assert_eq!(config.system_prompt, Some("You are a helpful assistant.".to_string()));
    }

    #[test]
    fn test_agent_config_with_tools() {
        let config = AgentConfig::new("test", "Test", "gpt-4")
            .with_tools(vec!["tool1".to_string(), "tool2".to_string()]);
        assert_eq!(config.tools.len(), 2);
        assert!(config.tools.contains(&"tool1".to_string()));
    }

    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig::new("test", "Test", "gpt-4");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_agent_config_deserialization() {
        let json = r#"{"id":"test","name":"Test","description":"","default_model":"gpt-4","max_turns":50,"timeout_seconds":300,"tools":[],"skills":[],"metadata":{}}"#;
        let config: AgentConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "test");
        assert_eq!(config.default_model, "gpt-4");
    }

    #[test]
    fn test_agent_config_clone() {
        let config = AgentConfig::new("test", "Test", "gpt-4")
            .with_system_prompt("System prompt")
            .with_tools(vec!["tool1".to_string()]);
        let cloned = config.clone();
        assert_eq!(cloned.id, config.id);
        assert_eq!(cloned.system_prompt, config.system_prompt);
        assert_eq!(cloned.tools, config.tools);
    }

    #[test]
    fn test_agent_config_debug() {
        let config = AgentConfig::new("test", "Test", "gpt-4");
        let debug = format!("{:?}", config);
        assert!(debug.contains("AgentConfig"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_agent_context_new() {
        let ctx = AgentContext::new("session-123");
        assert_eq!(ctx.session_id, "session-123");
        assert!(ctx.history.is_empty());
        assert!(ctx.data.is_empty());
    }

    #[test]
    fn test_agent_context_with_history() {
        let history = vec![Message::user("Hello"), Message::assistant("Hi there!")];
        let ctx = AgentContext::new("session-123").with_history(history.clone());
        assert_eq!(ctx.history.len(), 2);
    }

    #[test]
    fn test_agent_context_clone() {
        let ctx = AgentContext::new("session-123")
            .with_history(vec![Message::user("Hello")]);
        let cloned = ctx.clone();
        assert_eq!(cloned.session_id, ctx.session_id);
        assert_eq!(cloned.history.len(), ctx.history.len());
    }

    #[test]
    fn test_agent_context_abort_signal() {
        let ctx = AgentContext::new("session-123");
        assert!(!ctx.abort_signal.is_aborted());
        ctx.abort_signal.abort();
        assert!(ctx.abort_signal.is_aborted());
    }

    #[test]
    fn test_agent_response_serialization() {
        let response = AgentResponse {
            message: Message::assistant("Hello!"),
            is_complete: true,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
            usage: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("is_complete"));
    }

    #[test]
    fn test_agent_response_deserialization() {
        let json = r#"{"message":{"role":"assistant","content":[{"type":"text","text":"Hello"}]},"is_complete":true,"tool_calls":[],"metadata":{}}"#;
        let response: AgentResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_complete);
    }

    #[test]
    fn test_agent_response_clone() {
        let response = AgentResponse {
            message: Message::assistant("Hello!"),
            is_complete: false,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
            usage: None,
        };
        let cloned = response.clone();
        assert_eq!(cloned.is_complete, response.is_complete);
    }

    #[test]
    fn test_agent_response_debug() {
        let response = AgentResponse {
            message: Message::assistant("Hello!"),
            is_complete: true,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
            usage: None,
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("AgentResponse"));
    }

    #[test]
    fn test_default_max_turns() {
        assert_eq!(default_max_turns(), 50);
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 300);
    }
}
