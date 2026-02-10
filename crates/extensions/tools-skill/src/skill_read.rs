//! Skill read tool - read files from within a skill directory.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;

use autohands_protocols::error::ToolError;
use autohands_protocols::skill::SkillLoader;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct SkillReadParams {
    /// Skill ID.
    skill_id: String,
    /// Relative path within the skill directory.
    path: String,
}

/// Tool for reading files from within a skill's directory.
///
/// Some skills come with additional resources like templates,
/// examples, or reference documentation. This tool allows the
/// Agent to access those resources when needed.
pub struct SkillReadTool {
    definition: ToolDefinition,
    loader: Arc<RwLock<dyn SkillLoader>>,
}

impl SkillReadTool {
    pub fn new(loader: Arc<RwLock<dyn SkillLoader>>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The ID of the skill containing the file"
                },
                "path": {
                    "type": "string",
                    "description": "Relative path to the file within the skill directory (e.g., 'templates/report.md', 'examples/config.yaml')"
                }
            },
            "required": ["skill_id", "path"]
        });

        Self {
            definition: ToolDefinition::new(
                "skill_read",
                "Read Skill Resource",
                "Read a file from within a skill's directory. Use this to access templates, examples, or reference documentation that comes with a skill.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            loader,
        }
    }
}

#[async_trait]
impl Tool for SkillReadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: SkillReadParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // Load skill to get base_dir
        let loader = self.loader.read().await;
        let skill = loader
            .load(&params.skill_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to load skill '{}': {}", params.skill_id, e)))?;

        // Get base directory from metadata
        let base_dir = skill
            .definition
            .metadata
            .get("base_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!(
                    "Skill '{}' does not have a base directory (single-file skill)",
                    params.skill_id
                ))
            })?;

        let base_path = PathBuf::from(base_dir);
        let target_path = base_path.join(&params.path);

        // Security check: ensure path doesn't escape skill directory
        let canonical_base = base_path.canonicalize().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to resolve skill directory: {}", e))
        })?;

        let canonical_target = target_path.canonicalize().map_err(|e| {
            ToolError::ExecutionFailed(format!("File not found: {} ({})", params.path, e))
        })?;

        if !canonical_target.starts_with(&canonical_base) {
            return Err(ToolError::ExecutionFailed(
                "Access denied: path escapes skill directory".to_string(),
            ));
        }

        // Read file
        let content = tokio::fs::read_to_string(&canonical_target)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let mut output = format!(
            "# Skill Resource: {}/{}\n\n",
            params.skill_id, params.path
        );
        output.push_str("```\n");
        output.push_str(&content);
        output.push_str("\n```");

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
#[path = "skill_read_tests.rs"]
mod tests;
