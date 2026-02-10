use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_skill_source_priority() {
    assert!(SkillSource::Bundled.priority() < SkillSource::Managed(PathBuf::new()).priority());
    assert!(
        SkillSource::Managed(PathBuf::new()).priority()
            < SkillSource::Workspace(PathBuf::new()).priority()
    );
}

#[tokio::test]
async fn test_loader_new() {
    let loader = DynamicSkillLoader::new();
    // Should have at least managed source if home dir exists
    assert!(!loader.sources().is_empty() || dirs::home_dir().is_none());
}

#[tokio::test]
async fn test_loader_with_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let loader = DynamicSkillLoader::new().with_workspace(temp_dir.path().to_path_buf());

    // Should have workspace source
    assert!(loader.sources().iter().any(|s| matches!(s, SkillSource::Workspace(_))));
}

#[tokio::test]
async fn test_empty_loader_list() {
    let loader = DynamicSkillLoader::new();
    let skills = loader.list().await.unwrap();
    assert!(skills.is_empty());
}

#[tokio::test]
async fn test_load_nonexistent_skill() {
    let loader = DynamicSkillLoader::new();
    let result = loader.load("nonexistent").await;
    assert!(matches!(result, Err(SkillError::NotFound(_))));
}
