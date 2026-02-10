//! Cron job status tool.

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for cron_status tool.
#[derive(Debug, Deserialize)]
struct CronStatusParams {
    /// ID or name of the task to check.
    id: String,
}

/// Get cron job status tool implementation.
pub struct CronStatusTool {
    definition: ToolDefinition,
}

impl CronStatusTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "ID or name of the scheduled task"
                },
                "include_history": {
                    "type": "boolean",
                    "description": "If true, include recent execution history"
                }
            },
            "required": ["id"]
        });

        Self {
            definition: ToolDefinition::new(
                "cron_status",
                "Cron Job Status",
                "Get detailed status and execution history of a scheduled task",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for CronStatusTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronStatusTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CronStatusParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // In a real implementation, this would fetch task status from the scheduler
        // For now, we return a not-found response

        Err(ToolError::ResourceNotFound(format!(
            "Scheduled task '{}' not found",
            params.id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_context() -> ToolContext {
        ToolContext::new("test", PathBuf::from("/tmp"))
    }

    #[test]
    fn test_tool_definition() {
        let tool = CronStatusTool::new();
        assert_eq!(tool.definition().id, "cron_status");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_status_not_found() {
        let tool = CronStatusTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "id": "nonexistent"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ResourceNotFound(msg) => {
                assert!(msg.contains("nonexistent"));
            }
            e => panic!("Expected ResourceNotFound, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_missing_id() {
        let tool = CronStatusTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }
}
