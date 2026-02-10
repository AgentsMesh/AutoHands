//! Skill info tool.

use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::registry::SkillRegistry;

/// Tool to get detailed info about a skill.
pub struct SkillInfoTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillInfoTool {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_info",
            "skill_info",
            "Get detailed information about a specific skill",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID to get info for"
                }
            },
            "required": ["skill_id"]
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillInfoTool {
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

        match self.registry.get(skill_id).await {
            Some(skill) => {
                let info = serde_json::json!({
                    "id": skill.definition.id,
                    "name": skill.definition.name,
                    "description": skill.definition.description,
                    "category": skill.definition.category,
                    "tags": skill.definition.tags,
                    "priority": skill.definition.priority,
                    "required_tools": skill.definition.required_tools,
                    "variables": skill.definition.variables,
                    "enabled": skill.definition.enabled,
                    "content_preview": skill.content.chars().take(200).collect::<String>(),
                });
                Ok(ToolResult::success(serde_json::to_string_pretty(&info).unwrap()))
            }
            None => Ok(ToolResult::error(format!("Skill not found: {}", skill_id))),
        }
    }
}
