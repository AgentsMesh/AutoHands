//! Skill load tool - load a skill's expert guidance.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;

use autohands_protocols::error::ToolError;
use autohands_protocols::skill::SkillLoader;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct SkillLoadParams {
    /// Skill ID to load.
    skill_id: String,
}

/// Tool for loading a skill's content.
///
/// This is the core tool that allows the Agent to dynamically
/// activate a skill's expert guidance. When loaded, the skill's
/// content (which contains expert instructions, workflows, and
/// best practices) is returned for the Agent to follow.
pub struct SkillLoadTool {
    definition: ToolDefinition,
    loader: Arc<RwLock<dyn SkillLoader>>,
}

impl SkillLoadTool {
    pub fn new(loader: Arc<RwLock<dyn SkillLoader>>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The ID of the skill to load (e.g., 'code-review', 'security-audit')"
                }
            },
            "required": ["skill_id"]
        });

        Self {
            definition: ToolDefinition::new(
                "skill_load",
                "Load Skill",
                "Load a skill's expert guidance to enhance your capabilities for the current task. The loaded content contains expert instructions, workflows, and best practices that you should follow.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            loader,
        }
    }
}

#[async_trait]
impl Tool for SkillLoadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: SkillLoadParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let loader = self.loader.read().await;
        let skill = loader
            .load(&params.skill_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to load skill '{}': {}", params.skill_id, e)))?;

        // Format the skill content with metadata header
        let mut output = String::new();
        output.push_str(&format!("# Skill Activated: {}\n\n", skill.definition.name));
        output.push_str(&format!("**ID**: {}\n", skill.definition.id));
        output.push_str(&format!("**Description**: {}\n", skill.definition.description));

        if let Some(cat) = &skill.definition.category {
            output.push_str(&format!("**Category**: {}\n", cat));
        }

        if !skill.definition.tags.is_empty() {
            output.push_str(&format!("**Tags**: {}\n", skill.definition.tags.join(", ")));
        }

        if !skill.definition.required_tools.is_empty() {
            output.push_str(&format!(
                "**Required Tools**: {}\n",
                skill.definition.required_tools.join(", ")
            ));
        }

        output.push_str("\n---\n\n");
        output.push_str("## Expert Guidance\n\n");
        output.push_str("Follow the instructions below to complete the task:\n\n");
        output.push_str(&skill.content);

        // Add note about skill resources if base_dir exists
        if skill.definition.metadata.contains_key("base_dir") {
            output.push_str("\n\n---\n\n");
            output.push_str("**Note**: This skill has additional resources. Use `skill_read` to access files within the skill directory if needed.");
        }

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
            let mut def = SkillDefinition::new("code-review", "Code Review Expert");
            def.description = "Expert code reviewer".to_string();
            def.category = Some("development".to_string());
            def.tags = vec!["review".to_string(), "quality".to_string()];
            def.required_tools = vec!["read_file".to_string(), "grep".to_string()];

            Self {
                skills: vec![Skill::new(
                    def,
                    r#"# Code Review Expert

You are an expert code reviewer. Follow these steps:

## 1. Understand the Context
- Read the files to be reviewed
- Understand the project structure

## 2. Check for Issues
- Security vulnerabilities
- Performance problems
- Code style violations

## 3. Provide Feedback
- Be constructive
- Suggest improvements
- Highlight good practices
"#,
                )],
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
    async fn test_skill_load() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillLoadTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool
            .execute(serde_json::json!({"skill_id": "code-review"}), ctx)
            .await
            .unwrap();

        assert!(result.content.contains("Skill Activated: Code Review Expert"));
        assert!(result.content.contains("Expert Guidance"));
        assert!(result.content.contains("Check for Issues"));
        assert!(result.content.contains("Security vulnerabilities"));
    }

    #[tokio::test]
    async fn test_skill_load_not_found() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillLoadTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool
            .execute(serde_json::json!({"skill_id": "nonexistent"}), ctx)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_skill_load_missing_param() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader::new()));
        let tool = SkillLoadTool::new(loader);
        let ctx = ToolContext::new("test", PathBuf::from("."));

        let result = tool.execute(serde_json::json!({}), ctx).await;
        assert!(result.is_err());
    }
}
