//! Completion request types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tool::ToolDefinition;
use crate::types::{Message, Metadata};

/// Request for a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model to use.
    pub model: String,

    /// Messages in the conversation.
    pub messages: Vec<Message>,

    /// System message (if supported separately).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Available tools.
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 - 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences.
    #[serde(default)]
    pub stop: Vec<String>,

    /// Tool choice mode.
    #[serde(default)]
    pub tool_choice: ToolChoice,

    /// Request timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

impl CompletionRequest {
    /// Create a new completion request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: Vec::new(),
            tool_choice: ToolChoice::Auto,
            timeout_seconds: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the system message.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the tools.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Tool choice mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide.
    #[default]
    Auto,

    /// Don't use tools.
    None,

    /// Must use a tool.
    Required,

    /// Force a specific tool.
    Tool { name: String },
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
