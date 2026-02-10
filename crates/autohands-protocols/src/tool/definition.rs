//! Tool definition types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Metadata, RiskLevel};

/// Definition of a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique identifier for the tool.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of what the tool does.
    pub description: String,

    /// JSON Schema for the parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_schema: Option<serde_json::Value>,

    /// Risk level for this tool.
    #[serde(default)]
    pub risk_level: RiskLevel,

    /// Whether this tool supports streaming output.
    #[serde(default)]
    pub supports_streaming: bool,

    /// Extension ID that provides this tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_id: Option<String>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            parameters_schema: None,
            risk_level: RiskLevel::Low,
            supports_streaming: false,
            extension_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the parameters schema.
    pub fn with_parameters_schema(mut self, schema: serde_json::Value) -> Self {
        self.parameters_schema = Some(schema);
        self
    }

    /// Set the risk level.
    pub fn with_risk_level(mut self, risk_level: RiskLevel) -> Self {
        self.risk_level = risk_level;
        self
    }

    /// Convert to OpenAI function calling format.
    pub fn to_openai_function(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.id,
                "description": self.description,
                "parameters": self.parameters_schema.clone().unwrap_or_else(empty_object_schema)
            }
        })
    }

    /// Convert to Anthropic tool format.
    pub fn to_anthropic_tool(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.id,
            "description": self.description,
            "input_schema": self.parameters_schema.clone().unwrap_or_else(empty_object_schema)
        })
    }
}

fn empty_object_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}

#[cfg(test)]
#[path = "definition_tests.rs"]
mod tests;
