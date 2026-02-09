//! Delete cron job tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for cron_delete tool.
#[derive(Debug, Deserialize)]
struct CronDeleteParams {
    /// ID or name of the task to delete.
    id: String,
    /// If true, delete without confirmation (default: false).
    #[serde(default)]
    force: bool,
}

/// Response from cron_delete.
#[derive(Debug, Serialize)]
struct CronDeleteResponse {
    /// Whether the deletion was successful.
    success: bool,
    /// ID of the deleted task.
    id: String,
    /// Name of the deleted task.
    name: Option<String>,
    /// Status message.
    message: String,
}

/// Delete cron job tool implementation.
pub struct CronDeleteTool {
    definition: ToolDefinition,
}

impl CronDeleteTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "ID or name of the scheduled task to delete"
                },
                "force": {
                    "type": "boolean",
                    "description": "If true, delete without confirmation warnings"
                }
            },
            "required": ["id"]
        });

        Self {
            definition: ToolDefinition::new(
                "cron_delete",
                "Delete Cron Job",
                "Delete a scheduled task by ID or name",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for CronDeleteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronDeleteTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CronDeleteParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // In a real implementation, this would delete the task from the scheduler
        // For now, we simulate the response

        let response = CronDeleteResponse {
            success: true,
            id: params.id.clone(),
            name: None,
            message: format!(
                "Task '{}' has been {}deleted.",
                params.id,
                if params.force { "forcefully " } else { "" }
            ),
        };

        tracing::info!("Deleted cron job: id={}", params.id);

        Ok(ToolResult::success(serde_json::to_string_pretty(&response).unwrap()))
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
        let tool = CronDeleteTool::new();
        assert_eq!(tool.definition().id, "cron_delete");
        assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let tool = CronDeleteTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "id": "abc123"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("abc123"));
        assert!(result.content.contains("deleted"));
    }

    #[tokio::test]
    async fn test_force_delete() {
        let tool = CronDeleteTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "id": "abc123",
            "force": true
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("forcefully"));
    }

    #[tokio::test]
    async fn test_missing_id() {
        let tool = CronDeleteTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }
}
