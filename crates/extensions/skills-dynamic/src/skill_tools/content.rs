//! Skill content tool.

use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::registry::SkillRegistry;

/// Tool to get the full content/prompt of a skill.
pub struct SkillContentTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillContentTool {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_content",
            "skill_content",
            "Get the full content/prompt of a skill for use as system prompt",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID to get content for"
                },
                "variables": {
                    "type": "object",
                    "description": "Variables to render in the skill template"
                }
            },
            "required": ["skill_id"]
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillContentTool {
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

        let skill = self.registry.get(skill_id).await.ok_or_else(|| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Skill not found: {}",
                skill_id
            ))
        })?;

        // Extract variables if provided
        let variables: std::collections::HashMap<String, String> = params
            .get("variables")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        // Render the skill content with variables
        let content = skill.render(&variables);

        Ok(ToolResult::success(content))
    }
}
