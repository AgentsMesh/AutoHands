//! Tool execution result types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Metadata;

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the execution was successful.
    pub success: bool,

    /// Output content.
    pub content: String,

    /// Structured output (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<serde_json::Value>,

    /// Error message if execution failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Additional metadata about the execution.
    #[serde(default)]
    pub metadata: Metadata,
}

impl ToolResult {
    /// Create a successful result with text content.
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            structured_output: None,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a successful result with structured output.
    pub fn success_json(content: impl Into<String>, output: serde_json::Value) -> Self {
        Self {
            success: true,
            content: content.into(),
            structured_output: Some(output),
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an error result.
    pub fn error(error: impl Into<String>) -> Self {
        let error_msg = error.into();
        Self {
            success: false,
            content: String::new(),
            structured_output: None,
            error: Some(error_msg),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the result.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// A streaming tool result chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultChunk {
    /// Chunk content.
    pub content: String,

    /// Whether this is the final chunk.
    pub is_final: bool,

    /// Error if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
