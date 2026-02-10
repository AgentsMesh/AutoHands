//! Create directory tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for create_directory tool.
#[derive(Debug, Deserialize)]
struct CreateDirParams {
    /// Path to the directory to create.
    path: String,
    /// Create parent directories if they don't exist.
    #[serde(default = "default_true")]
    parents: bool,
}

fn default_true() -> bool {
    true
}

/// Create directory tool implementation.
pub struct CreateDirectoryTool {
    definition: ToolDefinition,
}

impl CreateDirectoryTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to create"
                },
                "parents": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist (default: true)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "create_directory",
                "Create Directory",
                "Create a new directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for CreateDirectoryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CreateDirParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if path.exists() {
            return Ok(ToolResult::success(format!(
                "Directory already exists: {}",
                path.display()
            )));
        }

        if params.parents {
            tokio::fs::create_dir_all(&path).await?;
        } else {
            tokio::fs::create_dir(&path).await?;
        }

        Ok(ToolResult::success(format!(
            "Created directory: {}",
            path.display()
        )))
    }
}

fn resolve_path(path: &str, work_dir: &PathBuf) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        work_dir.join(p)
    }
}

#[cfg(test)]
#[path = "create_dir_tests.rs"]
mod tests;
