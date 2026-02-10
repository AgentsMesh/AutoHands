//! Edit file tool (search/replace).

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for edit_file tool.
#[derive(Debug, Deserialize)]
struct EditFileParams {
    /// Path to the file to edit.
    path: String,
    /// Text to search for.
    old_string: String,
    /// Text to replace with.
    new_string: String,
    /// Replace all occurrences.
    #[serde(default)]
    replace_all: bool,
}

/// Edit file tool implementation (search/replace).
pub struct EditFileTool {
    definition: ToolDefinition,
}

impl EditFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to search for (must be unique unless replace_all)"
                },
                "new_string": {
                    "type": "string",
                    "description": "Text to replace with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)"
                }
            },
            "required": ["path", "old_string", "new_string"]
        });

        Self {
            definition: ToolDefinition::new(
                "edit_file",
                "Edit File",
                "Edit a file by replacing text (search/replace)",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for EditFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: EditFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&path).await?;

        // Check uniqueness
        let count = content.matches(&params.old_string).count();
        if count == 0 {
            return Err(ToolError::ExecutionFailed(format!(
                "Text not found in file: {}",
                params.old_string
            )));
        }

        if count > 1 && !params.replace_all {
            return Err(ToolError::ExecutionFailed(format!(
                "Text found {} times. Use replace_all or provide more context.",
                count
            )));
        }

        // Perform replacement
        let new_content = if params.replace_all {
            content.replace(&params.old_string, &params.new_string)
        } else {
            content.replacen(&params.old_string, &params.new_string, 1)
        };

        tokio::fs::write(&path, &new_content).await?;

        Ok(ToolResult::success(format!(
            "Successfully edited {} ({} replacement{})",
            path.display(),
            count,
            if count > 1 { "s" } else { "" }
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
#[path = "edit_file_tests.rs"]
mod tests;
