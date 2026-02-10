//! Skill reload tool.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::loader::DynamicSkillLoader;
use crate::registry::SkillRegistry;

/// Tool to reload all skills (useful after manual file changes).
pub struct SkillReloadTool {
    definition: ToolDefinition,
    loader: Arc<RwLock<DynamicSkillLoader>>,
    registry: Arc<SkillRegistry>,
}

impl SkillReloadTool {
    pub fn new(loader: Arc<RwLock<DynamicSkillLoader>>, registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_reload",
            "skill_reload",
            "Reload all skills from disk",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {}
        }));

        Self { definition, loader, registry }
    }
}

#[async_trait]
impl Tool for SkillReloadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        use autohands_protocols::skill::SkillLoader;

        // Reload from loader
        let loader = self.loader.read().await;
        loader.reload().await.map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to reload skills: {}",
                e
            ))
        })?;

        // Update registry
        let skills = loader.list().await.map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to list skills: {}",
                e
            ))
        })?;

        // Clear and re-register
        self.registry.clear().await;
        for def in &skills {
            if let Ok(skill) = loader.load(&def.id).await {
                self.registry.register(skill).await;
            }
        }

        Ok(ToolResult::success(format!("Reloaded {} skills", skills.len())))
    }
}
