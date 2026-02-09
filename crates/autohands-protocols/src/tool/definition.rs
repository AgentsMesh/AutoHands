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
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_new() {
        let tool = ToolDefinition::new("test", "Test Tool", "A test tool");
        assert_eq!(tool.id, "test");
        assert_eq!(tool.name, "Test Tool");
        assert_eq!(tool.description, "A test tool");
        assert!(tool.parameters_schema.is_none());
        assert_eq!(tool.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_with_parameters_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        });
        let tool = ToolDefinition::new("test", "Test", "Test")
            .with_parameters_schema(schema.clone());
        assert_eq!(tool.parameters_schema, Some(schema));
    }

    #[test]
    fn test_with_risk_level() {
        let tool = ToolDefinition::new("test", "Test", "Test")
            .with_risk_level(RiskLevel::High);
        assert_eq!(tool.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_to_openai_function() {
        let tool = ToolDefinition::new("read_file", "Read File", "Read a file");
        let func = tool.to_openai_function();

        assert_eq!(func["type"], "function");
        assert_eq!(func["function"]["name"], "read_file");
        assert_eq!(func["function"]["description"], "Read a file");
    }

    #[test]
    fn test_to_anthropic_tool() {
        let tool = ToolDefinition::new("read_file", "Read File", "Read a file");
        let tool_json = tool.to_anthropic_tool();

        assert_eq!(tool_json["name"], "read_file");
        assert_eq!(tool_json["description"], "Read a file");
    }

    #[test]
    fn test_empty_object_schema() {
        let schema = empty_object_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
    }

    #[test]
    fn test_tool_serialization() {
        let tool = ToolDefinition::new("test", "Test", "Test tool");
        let json = serde_json::to_string(&tool).unwrap();
        let parsed: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
    }

    #[test]
    fn test_tool_definition_clone() {
        let tool = ToolDefinition::new("id", "name", "desc");
        let cloned = tool.clone();
        assert_eq!(cloned.id, "id");
        assert_eq!(cloned.name, "name");
    }

    #[test]
    fn test_tool_definition_debug() {
        let tool = ToolDefinition::new("debug_tool", "Debug", "For debugging");
        let debug = format!("{:?}", tool);
        assert!(debug.contains("ToolDefinition"));
        assert!(debug.contains("debug_tool"));
    }

    #[test]
    fn test_to_openai_function_with_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            },
            "required": ["query"]
        });
        let tool = ToolDefinition::new("search", "Search", "Search things")
            .with_parameters_schema(schema);
        let func = tool.to_openai_function();

        assert_eq!(func["function"]["parameters"]["type"], "object");
        assert!(func["function"]["parameters"]["properties"]["query"].is_object());
    }

    #[test]
    fn test_to_anthropic_tool_with_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path"}
            }
        });
        let tool = ToolDefinition::new("read", "Read", "Read file")
            .with_parameters_schema(schema);
        let tool_json = tool.to_anthropic_tool();

        assert!(tool_json["input_schema"]["properties"]["path"].is_object());
    }

    #[test]
    fn test_tool_definition_full() {
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), serde_json::json!("1.0"));

        let tool = ToolDefinition {
            id: "full_tool".to_string(),
            name: "Full Tool".to_string(),
            description: "A fully configured tool".to_string(),
            parameters_schema: Some(serde_json::json!({"type": "object"})),
            risk_level: RiskLevel::Medium,
            supports_streaming: true,
            extension_id: Some("my-extension".to_string()),
            metadata,
        };

        assert_eq!(tool.id, "full_tool");
        assert!(tool.supports_streaming);
        assert_eq!(tool.extension_id, Some("my-extension".to_string()));
        assert!(tool.metadata.contains_key("version"));
    }

    #[test]
    fn test_tool_definition_chaining() {
        let tool = ToolDefinition::new("chain", "Chain", "Chained")
            .with_risk_level(RiskLevel::High)
            .with_parameters_schema(serde_json::json!({"type": "object"}));

        assert_eq!(tool.risk_level, RiskLevel::High);
        assert!(tool.parameters_schema.is_some());
    }

    #[test]
    fn test_tool_definition_deserialization() {
        let json = r#"{
            "id": "test_id",
            "name": "Test Name",
            "description": "Test Desc"
        }"#;
        let tool: ToolDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(tool.id, "test_id");
        assert_eq!(tool.name, "Test Name");
        assert_eq!(tool.risk_level, RiskLevel::Low);  // default
    }

    #[test]
    fn test_tool_definition_serialization_skips_none() {
        let tool = ToolDefinition::new("test", "Test", "Desc");
        let json = serde_json::to_string(&tool).unwrap();
        // parameters_schema and extension_id should not be present
        assert!(!json.contains("parameters_schema"));
        assert!(!json.contains("extension_id"));
    }

    #[test]
    fn test_tool_with_empty_strings() {
        let tool = ToolDefinition::new("", "", "");
        assert!(tool.id.is_empty());
        assert!(tool.name.is_empty());
        assert!(tool.description.is_empty());
    }

    #[test]
    fn test_openai_function_preserves_required() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"a": {"type": "string"}},
            "required": ["a"]
        });
        let tool = ToolDefinition::new("test", "Test", "Test")
            .with_parameters_schema(schema);
        let func = tool.to_openai_function();
        assert!(func["function"]["parameters"]["required"].is_array());
    }
}
