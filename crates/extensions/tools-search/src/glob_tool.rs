//! Glob pattern matching tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct GlobParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

/// Glob pattern matching tool.
pub struct GlobTool {
    definition: ToolDefinition,
}

impl GlobTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., **/*.rs, src/**/*.ts)"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in"
                }
            },
            "required": ["pattern"]
        });

        Self {
            definition: ToolDefinition::new("glob", "Glob Search", "Find files matching a pattern")
                .with_parameters_schema(schema)
                .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GlobParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let base_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.work_dir.clone());

        let full_pattern = base_path.join(&params.pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let mut matches = Vec::new();
        for entry in glob::glob(&pattern_str)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?
        {
            match entry {
                Ok(path) => {
                    let relative = path
                        .strip_prefix(&base_path)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    matches.push(relative);
                }
                Err(e) => {
                    tracing::warn!("Glob error: {}", e);
                }
            }
        }

        if matches.is_empty() {
            Ok(ToolResult::success("No files found matching pattern"))
        } else {
            Ok(ToolResult::success(format!(
                "Found {} files:\n{}",
                matches.len(),
                matches.join("\n")
            )))
        }
    }
}

#[cfg(test)]
#[path = "glob_tool_tests.rs"]
mod tests;
