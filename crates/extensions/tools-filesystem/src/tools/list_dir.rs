//! List directory tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for list_directory tool.
#[derive(Debug, Deserialize)]
struct ListDirectoryParams {
    /// Path to the directory to list.
    path: String,
    /// Maximum depth to recurse (default: 1).
    #[serde(default = "default_depth")]
    depth: usize,
}

fn default_depth() -> usize {
    1
}

/// List directory tool implementation.
pub struct ListDirectoryTool {
    definition: ToolDefinition,
}

impl ListDirectoryTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum depth to recurse (default: 1)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "list_directory",
                "List Directory",
                "List contents of a directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for ListDirectoryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListDirectoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ListDirectoryParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "Directory not found: {}",
                path.display()
            )));
        }

        if !path.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(&path)
            .max_depth(params.depth)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(&path)
                .unwrap_or(entry.path());

            if relative.as_os_str().is_empty() {
                continue;
            }

            let prefix = if entry.file_type().is_dir() {
                "📁"
            } else {
                "📄"
            };

            entries.push(format!("{} {}", prefix, relative.display()));
        }

        Ok(ToolResult::success(entries.join("\n")))
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
#[path = "list_dir_tests.rs"]
mod tests;
