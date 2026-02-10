//! Skill list tool.

use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::registry::SkillRegistry;

/// Tool to list all available skills.
pub struct SkillListTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillListTool {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_list",
            "skill_list",
            "List all available dynamic skills",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "tag": {
                    "type": "string",
                    "description": "Filter by tag"
                },
                "category": {
                    "type": "string",
                    "description": "Filter by category"
                }
            }
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillListTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let tag = params.get("tag").and_then(|v| v.as_str());
        let category = params.get("category").and_then(|v| v.as_str());

        let skills = if let Some(t) = tag {
            self.registry.find_by_tag(t).await
        } else if let Some(c) = category {
            self.registry.find_by_category(c).await
        } else {
            let defs = self.registry.list().await;
            // Convert definitions to minimal representation
            let list: Vec<serde_json::Value> = defs
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "id": d.id,
                        "name": d.name,
                        "description": d.description,
                        "tags": d.tags,
                        "category": d.category,
                    })
                })
                .collect();

            return Ok(ToolResult::success(serde_json::to_string_pretty(&list).unwrap()));
        };

        let list: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.definition.id,
                    "name": s.definition.name,
                    "description": s.definition.description,
                    "tags": s.definition.tags,
                    "category": s.definition.category,
                })
            })
            .collect();

        Ok(ToolResult::success(serde_json::to_string_pretty(&list).unwrap()))
    }
}
