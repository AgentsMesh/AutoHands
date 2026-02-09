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
mod tests {
    use super::*;

    #[test]
    fn test_background_params_parsing() {
        let json = serde_json::json!({
            "action": "list"
        });
        let params: BackgroundParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "list");
        assert!(params.command.is_none());
        assert!(params.process_id.is_none());
        assert!(params.cwd.is_none());
    }

    #[test]
    fn test_background_params_spawn() {
        let json = serde_json::json!({
            "action": "spawn",
            "command": "sleep 10",
            "cwd": "/tmp"
        });
        let params: BackgroundParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "spawn");
        assert_eq!(params.command, Some("sleep 10".to_string()));
        assert_eq!(params.cwd, Some("/tmp".to_string()));
    }

    #[test]
    fn test_background_params_with_process_id() {
        let json = serde_json::json!({
            "action": "status",
            "process_id": "proc_123"
        });
        let params: BackgroundParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "status");
        assert_eq!(params.process_id, Some("proc_123".to_string()));
    }

    #[test]
    fn test_tool_definition() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        assert_eq!(tool.definition().id, "background");
        assert_eq!(tool.definition().risk_level, RiskLevel::High);
    }

    #[test]
    fn test_format_status_running() {
        let status = ProcessStatus::Running;
        assert_eq!(BackgroundTool::format_status(&status), "Running");
    }

    #[test]
    fn test_format_status_completed() {
        let status = ProcessStatus::Completed(0);
        assert_eq!(BackgroundTool::format_status(&status), "Completed (exit code: 0)");

        let status = ProcessStatus::Completed(1);
        assert_eq!(BackgroundTool::format_status(&status), "Completed (exit code: 1)");
    }

    #[test]
    fn test_format_status_failed() {
        let status = ProcessStatus::Failed("error message".to_string());
        assert_eq!(BackgroundTool::format_status(&status), "Failed: error message");
    }

    #[tokio::test]
    async fn test_list_empty() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "list"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("No background processes"));
    }

    #[tokio::test]
    async fn test_spawn_missing_command() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "spawn"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_status_missing_process_id() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "status"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_status_not_found() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "status",
            "process_id": "nonexistent"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ResourceNotFound(_) => {}
            e => panic!("Expected ResourceNotFound, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kill_missing_process_id() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "kill"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_missing_process_id() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "wait"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "invalid"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidParameters(msg) => assert!(msg.contains("Unknown action")),
            e => panic!("Expected InvalidParameters, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_spawn_and_list() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        // Spawn a process
        let spawn_params = serde_json::json!({
            "action": "spawn",
            "command": "sleep 60"
        });

        let result = tool.execute(spawn_params, ctx.clone()).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("Background process started"));

        // List processes
        let list_params = serde_json::json!({
            "action": "list"
        });

        let result = tool.execute(list_params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("sleep 60"));
    }

    #[tokio::test]
    async fn test_invalid_params() {
        let manager = Arc::new(BackgroundManager::new());
        let tool = BackgroundTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }
}
