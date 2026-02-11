//! Write file tool.

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use super::resolve_path_safe;

/// Parameters for write_file tool.
#[derive(Debug, Deserialize)]
struct WriteFileParams {
    /// Path to the file to write.
    path: String,
    /// Content to write.
    content: String,
}

/// Write file tool implementation.
pub struct WriteFileTool {
    definition: ToolDefinition,
}

impl WriteFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        });

        Self {
            definition: ToolDefinition::new(
                "write_file",
                "Write File",
                "Write content to a file (overwrites existing)",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WriteFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path_safe(&params.path, &ctx.work_dir)?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, &params.content).await?;

        Ok(ToolResult::success(format!(
            "Successfully wrote {} bytes to {}",
            params.content.len(),
            path.display()
        )))
    }
}

#[cfg(test)]
#[path = "write_file_tests.rs"]
mod tests;
