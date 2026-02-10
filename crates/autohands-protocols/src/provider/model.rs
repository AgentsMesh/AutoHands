//! Model definition types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::Metadata;

/// Definition of an LLM model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    /// Model identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Maximum context length in tokens.
    pub context_length: u32,

    /// Maximum output tokens.
    pub max_output_tokens: u32,

    /// Whether the model supports vision/images.
    #[serde(default)]
    pub supports_vision: bool,

    /// Whether the model supports tool/function calling.
    #[serde(default)]
    pub supports_tools: bool,

    /// Whether the model supports system messages.
    #[serde(default = "default_true")]
    pub supports_system: bool,

    /// Cost per 1M input tokens (USD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cost_per_million: Option<f64>,

    /// Cost per 1M output tokens (USD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_cost_per_million: Option<f64>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

fn default_true() -> bool {
    true
}

impl ModelDefinition {
    /// Create a new model definition.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            context_length: 128_000,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: None,
            output_cost_per_million: None,
            metadata: HashMap::new(),
        }
    }

    /// Set context length.
    pub fn with_context_length(mut self, length: u32) -> Self {
        self.context_length = length;
        self
    }

    /// Enable vision support.
    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }
}

/// Provider capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports streaming completions.
    pub streaming: bool,

    /// Supports tool/function calling.
    pub tool_calling: bool,

    /// Supports vision/image inputs.
    pub vision: bool,

    /// Supports JSON mode output.
    pub json_mode: bool,

    /// Supports prompt caching.
    pub prompt_caching: bool,

    /// Supports batching requests.
    pub batching: bool,

    /// Maximum concurrent requests.
    pub max_concurrent: Option<u32>,
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
