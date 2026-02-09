//! Skill list tool - discover available skills.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;

use autohands_protocols::error::ToolError;
use autohands_protocols::skill::SkillLoader;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct SkillListParams {
    /// Filter by tag (optional).
    #[serde(default)]
    tag: Option<String>,
    /// Filter by category (optional).
    #[serde(default)]
    category: Option<String>,
}

/// Tool for listing available skills.
///
/// This allows the Agent to discover what skills are available
/// and choose the most appropriate one for the current task.
pub struct SkillListTool {
    definition: ToolDefinition,
    loader: Arc<RwLock<dyn SkillLoader>>,
}

impl SkillListTool {
    pub fn new(loader: Arc<RwLock<dyn SkillLoader>>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "tag": {
                    "type": "string",
                    "description": "Filter skills by tag (e.g., 'development', 'security')"
                },
                "category": {
                    "type": "string",
                    "description": "Filter skills by category (e.g., 'devops', 'ai')"
                }
            }
        });

        Self {
            definition: ToolDefinition::new(
                "skill_list",
                "List Skills",
                "List available skills that can enhance your capabilities. Use this to discover what expert knowledge is available for the current task.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            loader,
        }
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
    ) -> Result<ToolResult, ToolError> {
        let params: SkillListParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let loader = self.loader.read().await;
        let skills = loader
            .list()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Filter skills
        let filtered: Vec<_> = skills
            .into_iter()
            .filter(|s| {
                // Filter by tag
                if let Some(ref tag) = params.tag {
                    if !s.tags.iter().any(|t| t.contains(tag)) {
                        return false;
                    }
                }
                // Filter by category
                if let Some(ref cat) = params.category {
                    if s.category.as_ref().map(|c| c.contains(cat)) != Some(true) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            return Ok(ToolResult::success("No skills available matching the criteria."));
        }

        // Format output
        let mut output = format!("Found {} available skills:\n\n", filtered.len());

        for skill in filtered {
            output.push_str(&format!("## {}\n", skill.name));
            output.push_str(&format!("- **ID**: `{}`\n", skill.id));
            output.push_str(&format!("- **Description**: {}\n", skill.description));
            if let Some(cat) = &skill.category {
                output.push_str(&format!("- **Category**: {}\n", cat));
            }
            if !skill.tags.is_empty() {
                output.push_str(&format!("- **Tags**: {}\n", skill.tags.join(", ")));
            }
            output.push('\n');
        }

        output.push_str("\nTo use a skill, call `skill_load` with the skill ID.");

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::skill::{Skill, SkillDefinition};
    use autohands_protocols::error::SkillError;
    use std::path::PathBuf;

    struct MockLoader {
        skills: Vec<Skill>,
    }

    impl MockLoader {
        fn new() -> Self {
            let mut def1 = SkillDefinition::new("code-review", "Code Review Expert");
            def1.description = "Expert code reviewer".to_string();
            def1.category = Some("development".to_string());
            def1.tags = vec!["review".to_string(), "quality".to_string()];

            let mut def2 = SkillDefinition::new("security-audit", "Security Audit");
            def2.description = "Security vulnerability scanner".to_string();
            def2.category = Some("security".to_string());
            def2.tags = vec!["security".to_string(), "audit".to_string()];

            Self {
                skills: vec![
                    Skill::new(def1, "Review code content"),
                    Skill::new(def2, "Audit security content"),
                ],
            }
        }
    }

    #[async_trait]
    impl SkillLoader for MockLoader {
        async fn load(&self, skill_id: &str) -> Result<Skill, SkillError> {
            self.skills
                .iter()
                .find(|s| s.definition.id == skill_id)
                .cloned()
                .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))
        }

        async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError> {
            Ok(self.skills.iter().map(|s| s.definition.clone()).collect())
        }

        async fn reload(&self) -> Result<(), SkillError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_skill_list_all() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillListTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.content.contains("Code Review Expert"));
        assert!(result.content.contains("Security Audit"));
    }

    #[tokio::test]
    async fn test_skill_list_filter_by_tag() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillListTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool
            .execute(serde_json::json!({"tag": "security"}), ctx)
            .await
            .unwrap();
        assert!(result.content.contains("Security Audit"));
        assert!(!result.content.contains("Code Review Expert"));
    }

    #[tokio::test]
    async fn test_skill_list_filter_by_category() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillListTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool
            .execute(serde_json::json!({"category": "development"}), ctx)
            .await
            .unwrap();
        assert!(result.content.contains("Code Review Expert"));
        assert!(!result.content.contains("Security Audit"));
    }
}
