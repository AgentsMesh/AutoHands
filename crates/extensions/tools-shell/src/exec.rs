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
#[path = "exec_tests.rs"]
mod tests;
