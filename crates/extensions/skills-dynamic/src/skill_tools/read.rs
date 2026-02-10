//! Skill file read tool.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::registry::SkillRegistry;

/// Tool to read files from a skill's directory.
pub struct SkillReadTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillReadTool {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_read",
            "skill_read",
            "Read a file from a skill's directory",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID"
                },
                "path": {
                    "type": "string",
                    "description": "Relative path within the skill directory"
                }
            },
            "required": ["skill_id", "path"]
        }));

        Self { definition, registry }
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
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let skill_id = params
            .get("skill_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "skill_id is required".to_string(),
                )
            })?;

        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "path is required".to_string(),
                )
            })?;

        let skill = self.registry.get(skill_id).await.ok_or_else(|| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Skill not found: {}",
                skill_id
            ))
        })?;

        // Get base directory from metadata
        let base_dir = skill
            .definition
            .metadata
            .get("base_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::ExecutionFailed(
                    "Skill has no base directory".to_string(),
                )
            })?;

        let full_path = PathBuf::from(base_dir).join(path);

        // Security check: ensure path is within skill directory
        let canonical_base = std::fs::canonicalize(base_dir).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to resolve base path: {}",
                e
            ))
        })?;
        let canonical_path = std::fs::canonicalize(&full_path).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to resolve path: {}",
                e
            ))
        })?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(autohands_protocols::error::ToolError::ExecutionFailed(
                "Path traversal detected".to_string(),
            ));
        }

        // Read the file
        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to read file: {}",
                e
            ))
        })?;

        Ok(ToolResult::success(content))
    }
}
