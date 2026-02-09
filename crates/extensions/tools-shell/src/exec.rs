//! Shell command execution tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for exec tool.
#[derive(Debug, Deserialize)]
struct ExecParams {
    /// Command to execute.
    command: String,
    /// Timeout in milliseconds (default: 120000).
    #[serde(default = "default_timeout")]
    timeout: u64,
    /// Working directory (optional).
    #[serde(default)]
    cwd: Option<String>,
}

fn default_timeout() -> u64 {
    120_000
}

/// Shell command execution tool.
pub struct ExecTool {
    definition: ToolDefinition,
}

impl ExecTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds (default: 120000)"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for the command"
                }
            },
            "required": ["command"]
        });

        Self {
            definition: ToolDefinition::new(
                "exec",
                "Execute Command",
                "Execute a shell command",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::High),
        }
    }
}

impl Default for ExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ExecParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let cwd = params
            .cwd
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ctx.work_dir.clone());

        // Determine shell based on platform
        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(flag)
            .arg(&params.command)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let duration = Duration::from_millis(params.timeout);

        let output = timeout(duration, cmd.output())
            .await
            .map_err(|_| ToolError::Timeout(params.timeout / 1000))?
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            result.push_str(&stderr);
        }

        if output.status.success() {
            Ok(ToolResult::success(result))
        } else {
            let code = output.status.code().unwrap_or(-1);
            Ok(ToolResult::error(format!(
                "Command failed with exit code {}\n{}",
                code, result
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_context(work_dir: std::path::PathBuf) -> ToolContext {
        ToolContext::new("test", work_dir)
    }

    #[test]
    fn test_tool_definition() {
        let tool = ExecTool::new();
        assert_eq!(tool.definition().id, "exec");
        assert_eq!(tool.definition().risk_level, RiskLevel::High);
    }

    #[test]
    fn test_default() {
        let tool = ExecTool::default();
        assert_eq!(tool.definition().id, "exec");
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 120_000);
    }

    #[tokio::test]
    async fn test_exec_echo() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "echo hello"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_exec_with_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("test.txt"), "content").unwrap();

        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "ls",
            "cwd": subdir.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_exec_failure() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "exit 1"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.as_ref().unwrap().contains("exit code 1"));
    }

    #[tokio::test]
    async fn test_exec_stderr() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "echo error >&2"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("error"));
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "sleep 10",
            "timeout": 100  // 100ms timeout
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::Timeout(_) => {}
            e => panic!("Expected Timeout, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_exec_invalid_params() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exec_multiline_output() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "command": "echo line1; echo line2; echo line3"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("line1"));
        assert!(result.content.contains("line2"));
        assert!(result.content.contains("line3"));
    }
}
