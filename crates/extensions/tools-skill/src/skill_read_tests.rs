use super::*;
use autohands_protocols::skill::{Skill, SkillDefinition};
use autohands_protocols::error::SkillError;
use tempfile::TempDir;

struct MockLoader {
    skill: Skill,
}

impl MockLoader {
    fn new(base_dir: &str) -> Self {
        let mut def = SkillDefinition::new("test-skill", "Test Skill");
        def.metadata.insert(
            "base_dir".to_string(),
            serde_json::json!(base_dir),
        );

        Self {
            skill: Skill::new(def, "Test content"),
        }
    }

    fn without_base_dir() -> Self {
        let def = SkillDefinition::new("single-file", "Single File Skill");
        Self {
            skill: Skill::new(def, "Single file content"),
        }
    }
}

#[async_trait]
impl SkillLoader for MockLoader {
    async fn load(&self, skill_id: &str) -> Result<Skill, SkillError> {
        if self.skill.definition.id == skill_id {
            Ok(self.skill.clone())
        } else {
            Err(SkillError::NotFound(skill_id.to_string()))
        }
    }

    async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError> {
        Ok(vec![self.skill.definition.clone()])
    }

    async fn reload(&self) -> Result<(), SkillError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_skill_read() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("template.md");
    tokio::fs::write(&file_path, "# Template\n\nHello world").await.unwrap();

    let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(
        MockLoader::new(&temp.path().to_string_lossy()),
    ));
    let tool = SkillReadTool::new(loader);
    let ctx = ToolContext::new("test", PathBuf::from("."));

    let result = tool
        .execute(
            serde_json::json!({
                "skill_id": "test-skill",
                "path": "template.md"
            }),
            ctx,
        )
        .await
        .unwrap();

    assert!(result.content.contains("Template"));
    assert!(result.content.contains("Hello world"));
}

#[tokio::test]
async fn test_skill_read_no_base_dir() {
    let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(
        MockLoader::without_base_dir(),
    ));
    let tool = SkillReadTool::new(loader);
    let ctx = ToolContext::new("test", PathBuf::from("."));

    let result = tool
        .execute(
            serde_json::json!({
                "skill_id": "single-file",
                "path": "anything.md"
            }),
            ctx,
        )
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not have a base directory"));
}

#[tokio::test]
async fn test_skill_read_path_escape() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("safe.txt"), "safe").await.unwrap();

    let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(
        MockLoader::new(&temp.path().to_string_lossy()),
    ));
    let tool = SkillReadTool::new(loader);
    let ctx = ToolContext::new("test", PathBuf::from("."));

    let result = tool
        .execute(
            serde_json::json!({
                "skill_id": "test-skill",
                "path": "../../../etc/passwd"
            }),
            ctx,
        )
        .await;

    // Should fail - either file not found or access denied
    assert!(result.is_err());
}
