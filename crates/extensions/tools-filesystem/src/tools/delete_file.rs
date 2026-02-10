//! Delete file tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for delete_file tool.
#[derive(Debug, Deserialize)]
struct DeleteFileParams {
    /// Path to the file or directory to delete.
    path: String,
    /// Delete directories recursively.
    #[serde(default)]
    recursive: bool,
}

/// Delete file tool implementation.
pub struct DeleteFileTool {
    definition: ToolDefinition,
}

impl DeleteFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Delete directories recursively (required for non-empty directories)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "delete_file",
                "Delete File",
                "Delete a file or directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::High),
        }
    }
}

impl Default for DeleteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DeleteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: DeleteFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "Path not found: {}",
                path.display()
            )));
        }

        let metadata = tokio::fs::metadata(&path).await?;

        if metadata.is_dir() {
            if params.recursive {
                tokio::fs::remove_dir_all(&path).await?;
            } else {
                tokio::fs::remove_dir(&path).await.map_err(|e| {
                    if e.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                        ToolError::ExecutionFailed(
                            "Directory not empty. Use recursive=true to delete non-empty directories.".to_string()
                        )
                    } else {
                        ToolError::from(e)
                    }
                })?;
            }
            Ok(ToolResult::success(format!(
                "Deleted directory: {}",
                path.display()
            )))
        } else {
            tokio::fs::remove_file(&path).await?;
            Ok(ToolResult::success(format!(
                "Deleted file: {}",
                path.display()
            )))
        }
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
#[path = "delete_file_tests.rs"]
mod tests;
