//! Tool trait definition.

use async_trait::async_trait;

use super::{ToolContext, ToolDefinition, ToolResult};
use crate::error::ToolError;
use crate::types::RiskLevel;

/// Core trait for tools.
///
/// Tools are executable units that agents can invoke to perform actions.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool definition.
    fn definition(&self) -> &ToolDefinition;

    /// Execute the tool with the given parameters.
    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// Validate the parameters before execution.
    fn validate(&self, params: &serde_json::Value) -> Result<(), ToolError> {
        let definition = self.definition();
        if let Some(schema) = &definition.parameters_schema {
            if schema.get("type") == Some(&serde_json::json!("object")) && !params.is_object() {
                return Err(ToolError::ValidationFailed(
                    "Parameters must be an object".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Returns the risk level of this tool.
    fn risk_level(&self) -> RiskLevel {
        self.definition().risk_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct MockTool {
        definition: ToolDefinition,
    }

    impl MockTool {
        fn new() -> Self {
            Self {
                definition: ToolDefinition::new("mock_tool", "Mock Tool", "A mock tool for testing"),
            }
        }

        fn with_schema(schema: serde_json::Value) -> Self {
            Self {
                definition: ToolDefinition::new("mock_tool", "Mock Tool", "A mock tool")
                    .with_parameters_schema(schema),
            }
        }

        fn with_risk_level(risk: RiskLevel) -> Self {
            Self {
                definition: ToolDefinition::new("mock_tool", "Mock Tool", "A mock tool")
                    .with_risk_level(risk),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("executed"))
        }
    }

    #[test]
    fn test_tool_definition() {
        let tool = MockTool::new();
        let def = tool.definition();
        assert_eq!(def.id, "mock_tool");
        assert_eq!(def.name, "Mock Tool");
    }

    #[test]
    fn test_tool_risk_level_default() {
        let tool = MockTool::new();
        assert_eq!(tool.risk_level(), RiskLevel::Low);
    }

    #[test]
    fn test_tool_risk_level_high() {
        let tool = MockTool::with_risk_level(RiskLevel::High);
        assert_eq!(tool.risk_level(), RiskLevel::High);
    }

    #[test]
    fn test_tool_risk_level_medium() {
        let tool = MockTool::with_risk_level(RiskLevel::Medium);
        assert_eq!(tool.risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn test_tool_validate_no_schema() {
        let tool = MockTool::new();
        let params = serde_json::json!({"key": "value"});
        assert!(tool.validate(&params).is_ok());
    }

    #[test]
    fn test_tool_validate_object_schema_with_object() {
        let schema = serde_json::json!({"type": "object", "properties": {}});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!({"key": "value"});
        assert!(tool.validate(&params).is_ok());
    }

    #[test]
    fn test_tool_validate_object_schema_with_non_object() {
        let schema = serde_json::json!({"type": "object", "properties": {}});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!("not an object");
        let result = tool.validate(&params);
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ValidationFailed(msg) => {
                assert!(msg.contains("must be an object"));
            }
            _ => panic!("Expected ValidationFailed error"),
        }
    }

    #[test]
    fn test_tool_validate_object_schema_with_array() {
        let schema = serde_json::json!({"type": "object"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!([1, 2, 3]);
        assert!(tool.validate(&params).is_err());
    }

    #[test]
    fn test_tool_validate_object_schema_with_number() {
        let schema = serde_json::json!({"type": "object"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!(42);
        assert!(tool.validate(&params).is_err());
    }

    #[test]
    fn test_tool_validate_object_schema_with_null() {
        let schema = serde_json::json!({"type": "object"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::Value::Null;
        assert!(tool.validate(&params).is_err());
    }

    #[test]
    fn test_tool_validate_object_schema_with_bool() {
        let schema = serde_json::json!({"type": "object"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!(true);
        assert!(tool.validate(&params).is_err());
    }

    #[test]
    fn test_tool_validate_non_object_schema() {
        let schema = serde_json::json!({"type": "string"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!("a string");
        // Non-object schema doesn't trigger validation error
        assert!(tool.validate(&params).is_ok());
    }

    #[test]
    fn test_tool_validate_empty_object() {
        let schema = serde_json::json!({"type": "object"});
        let tool = MockTool::with_schema(schema);
        let params = serde_json::json!({});
        assert!(tool.validate(&params).is_ok());
    }

    #[tokio::test]
    async fn test_tool_execute() {
        let tool = MockTool::new();
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let params = serde_json::json!({});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        assert_eq!(tool_result.content, "executed");
    }
}
