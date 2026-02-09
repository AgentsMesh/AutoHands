//! List cron jobs tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for cron_list tool.
#[derive(Debug, Deserialize)]
struct CronListParams {
    /// Filter by task name pattern (optional).
    #[serde(default)]
    filter: Option<String>,
    /// Only show enabled tasks (optional).
    #[serde(default)]
    enabled_only: bool,
    /// Maximum number of results to return (optional).
    #[serde(default)]
    limit: Option<usize>,
}

/// A scheduled task in the list.
#[derive(Debug, Serialize)]
struct CronTask {
    /// Unique ID of the task.
    id: String,
    /// Name of the task.
    name: String,
    /// Cron schedule expression.
    schedule: String,
    /// Command or prompt to execute.
    command: String,
    /// Whether the task is enabled.
    enabled: bool,
    /// When the task will next run.
    next_run: Option<String>,
    /// When the task last ran.
    last_run: Option<String>,
    /// Number of times the task has run.
    run_count: u32,
}

/// Response from cron_list.
#[derive(Debug, Serialize)]
struct CronListResponse {
    /// List of scheduled tasks.
    tasks: Vec<CronTask>,
    /// Total number of tasks.
    total: usize,
    /// Number of enabled tasks.
    enabled: usize,
    /// Number of disabled tasks.
    disabled: usize,
}

/// List cron jobs tool implementation.
pub struct CronListTool {
    definition: ToolDefinition,
}

impl CronListTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Optional filter pattern for task names (supports wildcards)"
                },
                "enabled_only": {
                    "type": "boolean",
                    "description": "If true, only show enabled tasks"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return"
                }
            }
        });

        Self {
            definition: ToolDefinition::new(
                "cron_list",
                "List Cron Jobs",
                "List all scheduled tasks with their status and next run times",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for CronListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronListTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CronListParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // In a real implementation, this would fetch tasks from the scheduler
        // For now, we return an empty list with the filter info

        let response = CronListResponse {
            tasks: vec![],
            total: 0,
            enabled: 0,
            disabled: 0,
        };

        let mut output = serde_json::to_string_pretty(&response).unwrap();

        if params.filter.is_some() || params.enabled_only || params.limit.is_some() {
            output.push_str("\n\n[Filters applied: ");
            let mut filters = vec![];
            if let Some(ref f) = params.filter {
                filters.push(format!("name={}", f));
            }
            if params.enabled_only {
                filters.push("enabled_only=true".to_string());
            }
            if let Some(l) = params.limit {
                filters.push(format!("limit={}", l));
            }
            output.push_str(&filters.join(", "));
            output.push(']');
        }

        Ok(ToolResult::success(output))
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
        let tool = CronListTool::new();
        assert_eq!(tool.definition().id, "cron_list");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_list_all_jobs() {
        let tool = CronListTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("tasks"));
        assert!(result.content.contains("total"));
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let tool = CronListTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "filter": "backup*",
            "enabled_only": true,
            "limit": 10
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("Filters applied"));
        assert!(result.content.contains("name=backup*"));
    }
}
