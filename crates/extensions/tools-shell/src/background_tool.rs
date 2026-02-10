//! Background process management tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::background::{BackgroundManager, ProcessStatus};

/// Parameters for background tool.
#[derive(Debug, Deserialize)]
struct BackgroundParams {
    /// Action: spawn, status, list, kill, wait
    action: String,
    /// Command to run (for spawn)
    #[serde(default)]
    command: Option<String>,
    /// Process ID (for status, kill, wait)
    #[serde(default)]
    process_id: Option<String>,
    /// Working directory (for spawn)
    #[serde(default)]
    cwd: Option<String>,
}

/// Background process management tool.
pub struct BackgroundTool {
    definition: ToolDefinition,
    manager: Arc<BackgroundManager>,
}

impl BackgroundTool {
    pub fn new(manager: Arc<BackgroundManager>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "status", "list", "kill", "wait"],
                    "description": "Action to perform"
                },
                "command": {
                    "type": "string",
                    "description": "Command to run in background (for spawn)"
                },
                "process_id": {
                    "type": "string",
                    "description": "Process ID (for status, kill, wait)"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory (for spawn)"
                }
            },
            "required": ["action"]
        });

        Self {
            definition: ToolDefinition::new(
                "background",
                "Background Process",
                "Manage background processes",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::High),
            manager,
        }
    }

    fn format_status(status: &ProcessStatus) -> String {
        match status {
            ProcessStatus::Running => "Running".to_string(),
            ProcessStatus::Completed(code) => format!("Completed (exit code: {})", code),
            ProcessStatus::Failed(msg) => format!("Failed: {}", msg),
        }
    }
}

#[async_trait]
impl Tool for BackgroundTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: BackgroundParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        match params.action.as_str() {
            "spawn" => {
                let command = params
                    .command
                    .ok_or_else(|| ToolError::InvalidParameters("command required".into()))?;

                let id = self
                    .manager
                    .spawn(&command, params.cwd.as_deref())
                    .map_err(|e| ToolError::ExecutionFailed(e))?;

                Ok(ToolResult::success(format!(
                    "Background process started: {}",
                    id
                )))
            }
            "status" => {
                let id = params
                    .process_id
                    .ok_or_else(|| ToolError::InvalidParameters("process_id required".into()))?;

                let status = self.manager.status(&id).ok_or_else(|| {
                    ToolError::ResourceNotFound(format!("Process not found: {}", id))
                })?;

                Ok(ToolResult::success(Self::format_status(&status)))
            }
            "list" => {
                let processes = self.manager.list();
                if processes.is_empty() {
                    Ok(ToolResult::success("No background processes"))
                } else {
                    let list: Vec<String> = processes
                        .iter()
                        .map(|(id, cmd, status)| {
                            format!("{}: {} [{}]", id, cmd, Self::format_status(status))
                        })
                        .collect();
                    Ok(ToolResult::success(list.join("\n")))
                }
            }
            "kill" => {
                let id = params
                    .process_id
                    .ok_or_else(|| ToolError::InvalidParameters("process_id required".into()))?;

                self.manager
                    .kill(&id)
                    .map_err(|e| ToolError::ExecutionFailed(e))?;

                Ok(ToolResult::success(format!("Process killed: {}", id)))
            }
            "wait" => {
                let id = params
                    .process_id
                    .ok_or_else(|| ToolError::InvalidParameters("process_id required".into()))?;

                let exit_code = self
                    .manager
                    .wait(&id)
                    .map_err(|e| ToolError::ExecutionFailed(e))?;

                Ok(ToolResult::success(format!(
                    "Process completed with exit code: {}",
                    exit_code
                )))
            }
            _ => Err(ToolError::InvalidParameters(format!(
                "Unknown action: {}",
                params.action
            ))),
        }
    }
}

#[cfg(test)]
#[path = "background_tool_tests.rs"]
mod tests;
