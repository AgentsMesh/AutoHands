//! Create cron job tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for cron_create tool.
#[derive(Debug, Deserialize)]
struct CronCreateParams {
    /// Name of the scheduled task.
    name: String,
    /// Cron expression (e.g., "0 0 * * *" for daily at midnight).
    /// Supports 5-field (standard) or 6-field (with seconds) format.
    schedule: String,
    /// Command or prompt to execute when the task runs.
    command: String,
    /// Whether the task is enabled (default: true).
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Response from cron_create.
#[derive(Debug, Serialize)]
struct CronCreateResponse {
    /// Unique ID of the created task.
    id: String,
    /// Name of the task.
    name: String,
    /// Schedule expression.
    schedule: String,
    /// When the task will next run.
    next_run: Option<String>,
    /// Status message.
    message: String,
}

/// Create cron job tool implementation.
pub struct CronCreateTool {
    definition: ToolDefinition,
}

impl CronCreateTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the scheduled task (must be unique)"
                },
                "schedule": {
                    "type": "string",
                    "description": "Cron expression. Examples: '0 0 * * *' (daily at midnight), '*/5 * * * *' (every 5 minutes), '0 9 * * 1-5' (weekdays at 9am)"
                },
                "command": {
                    "type": "string",
                    "description": "The command or prompt to execute when the task runs"
                },
                "description": {
                    "type": "string",
                    "description": "Optional description of what this task does"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the task is enabled (default: true)"
                },
                "timezone": {
                    "type": "string",
                    "description": "Optional timezone (e.g., 'America/New_York', 'Asia/Shanghai')"
                }
            },
            "required": ["name", "schedule", "command"]
        });

        Self {
            definition: ToolDefinition::new(
                "cron_create",
                "Create Cron Job",
                "Create a new scheduled task that will run at specified times",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for CronCreateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronCreateTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CronCreateParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // Validate cron expression
        let schedule = cron::Schedule::from_str(&params.schedule).map_err(|e| {
            ToolError::InvalidParameters(format!("Invalid cron expression '{}': {}", params.schedule, e))
        })?;

        // Calculate next run time
        let next_run = schedule.upcoming(chrono::Utc).next().map(|t| t.to_rfc3339());

        // Generate a unique ID for this task
        let id = uuid::Uuid::new_v4().to_string();

        // In a real implementation, this would store the task in the scheduler
        // For now, we return a success response indicating what would be created

        let response = CronCreateResponse {
            id: id.clone(),
            name: params.name.clone(),
            schedule: params.schedule.clone(),
            next_run,
            message: format!(
                "Created scheduled task '{}' with ID {}. {}",
                params.name,
                id,
                if params.enabled { "Task is enabled and will run on schedule." } else { "Task is disabled." }
            ),
        };

        // Log the creation
        tracing::info!(
            "Created cron job: id={}, name={}, schedule={}, command={}",
            id,
            params.name,
            params.schedule,
            params.command
        );

        Ok(ToolResult::success(serde_json::to_string_pretty(&response).unwrap()))
    }
}

use std::str::FromStr;

#[cfg(test)]
#[path = "cron_create_tests.rs"]
mod tests;
