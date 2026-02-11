//! Move/rename file tool.

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use super::resolve_path_safe;

/// Parameters for move_file tool.
#[derive(Debug, Deserialize)]
struct MoveFileParams {
    /// Source path.
    source: String,
    /// Destination path.
    destination: String,
    /// Overwrite destination if it exists.
    #[serde(default)]
    overwrite: bool,
}

/// Move/rename file tool implementation.
pub struct MoveFileTool {
    definition: ToolDefinition,
}

impl MoveFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source file or directory path"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination path"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Overwrite destination if it exists (default: false)"
                }
            },
            "required": ["source", "destination"]
        });

        Self {
            definition: ToolDefinition::new(
                "move_file",
                "Move File",
                "Move or rename a file or directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for MoveFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MoveFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MoveFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let source = resolve_path_safe(&params.source, &ctx.work_dir)?;
        let destination = resolve_path_safe(&params.destination, &ctx.work_dir)?;

        if !source.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "Source not found: {}",
                source.display()
            )));
        }

        if destination.exists() && !params.overwrite {
            return Err(ToolError::ExecutionFailed(format!(
                "Destination already exists: {}. Use overwrite=true to replace.",
                destination.display()
            )));
        }

        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Remove destination if overwriting
        if destination.exists() && params.overwrite {
            if destination.is_dir() {
                tokio::fs::remove_dir_all(&destination).await?;
            } else {
                tokio::fs::remove_file(&destination).await?;
            }
        }

        tokio::fs::rename(&source, &destination).await?;

        Ok(ToolResult::success(format!(
            "Moved {} to {}",
            source.display(),
            destination.display()
        )))
    }
}

#[cfg(test)]
#[path = "move_file_tests.rs"]
mod tests;
