//! Shell session tool for persistent sessions.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::session::SessionManager;

/// Parameters for session tool.
#[derive(Debug, Deserialize)]
struct SessionParams {
    /// Action to perform: create, execute, list, kill
    action: String,
    /// Session ID (for execute, kill)
    #[serde(default)]
    session_id: Option<String>,
    /// Command to execute (for execute action)
    #[serde(default)]
    command: Option<String>,
    /// Timeout in milliseconds (for execute action)
    #[serde(default = "default_timeout")]
    timeout: u64,
}

fn default_timeout() -> u64 {
    30_000
}

/// Shell session tool for persistent sessions.
pub struct SessionTool {
    definition: ToolDefinition,
    manager: Arc<SessionManager>,
}

impl SessionTool {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "execute", "list", "kill"],
                    "description": "Action to perform"
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID (required for execute, kill)"
                },
                "command": {
                    "type": "string",
                    "description": "Command to execute (required for execute action)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds (default: 30000)"
                }
            },
            "required": ["action"]
        });

        Self {
            definition: ToolDefinition::new(
                "shell_session",
                "Shell Session",
                "Manage persistent shell sessions",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::High),
            manager,
        }
    }
}

#[async_trait]
impl Tool for SessionTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: SessionParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        match params.action.as_str() {
            "create" => {
                let id = self
                    .manager
                    .create_session(None)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                Ok(ToolResult::success(format!("Session created: {}", id)))
            }
            "execute" => {
                let session_id = params
                    .session_id
                    .ok_or_else(|| ToolError::InvalidParameters("session_id required".into()))?;
                let command = params
                    .command
                    .ok_or_else(|| ToolError::InvalidParameters("command required".into()))?;

                let output = self
                    .manager
                    .execute_in_session(&session_id, &command, params.timeout)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

                Ok(ToolResult::success(output))
            }
            "list" => {
                let sessions = self.manager.list_sessions().await;
                if sessions.is_empty() {
                    Ok(ToolResult::success("No active sessions"))
                } else {
                    Ok(ToolResult::success(format!(
                        "Active sessions:\n{}",
                        sessions.join("\n")
                    )))
                }
            }
            "kill" => {
                let session_id = params
                    .session_id
                    .ok_or_else(|| ToolError::InvalidParameters("session_id required".into()))?;

                self.manager
                    .kill_session(&session_id)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

                Ok(ToolResult::success(format!("Session killed: {}", session_id)))
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
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30_000);
    }

    #[test]
    fn test_session_params_parsing() {
        let json = serde_json::json!({
            "action": "create"
        });
        let params: SessionParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "create");
        assert!(params.session_id.is_none());
        assert!(params.command.is_none());
        assert_eq!(params.timeout, 30_000);
    }

    #[test]
    fn test_session_params_with_all_fields() {
        let json = serde_json::json!({
            "action": "execute",
            "session_id": "session_1",
            "command": "echo hello",
            "timeout": 60000
        });
        let params: SessionParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "execute");
        assert_eq!(params.session_id, Some("session_1".to_string()));
        assert_eq!(params.command, Some("echo hello".to_string()));
        assert_eq!(params.timeout, 60000);
    }

    #[test]
    fn test_tool_definition() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        assert_eq!(tool.definition().id, "shell_session");
        assert_eq!(tool.definition().risk_level, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "create"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("Session created"));
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "list"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("No active sessions"));
    }

    #[tokio::test]
    async fn test_execute_missing_session_id() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "execute",
            "command": "echo hello"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_missing_command() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "execute",
            "session_id": "session_1"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_kill_missing_session_id() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({
            "action": "kill"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
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
    async fn test_invalid_params() {
        let manager = Arc::new(SessionManager::new());
        let tool = SessionTool::new(manager);
        let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }
}
