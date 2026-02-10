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
