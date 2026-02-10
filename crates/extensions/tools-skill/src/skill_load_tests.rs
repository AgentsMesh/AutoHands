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
