    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str, id: &str) {
        let content = format!(
            r#"---
id: {}
name: {}
description: Test skill
---

Test content for {}.
"#,
            id, name, name
        );

        fs::write(dir.join(format!("{}.markdown", id)), content).unwrap();
    }

    fn create_directory_skill(dir: &Path, id: &str) {
        let skill_dir = dir.join(id);
        fs::create_dir_all(&skill_dir).unwrap();

        let content = format!(
            r#"---
id: {}
name: {} Skill
description: A directory skill
---

Directory skill content.
"#,
            id,
            id.to_uppercase()
        );

        fs::write(skill_dir.join("SKILL.markdown"), content).unwrap();
    }

    #[tokio::test]
    async fn test_load_single_file_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_test_skill(dir, "Test One", "test-one");
        create_test_skill(dir, "Test Two", "test-two");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);

        let ids: Vec<&str> = skills.iter().map(|s| s.definition.id.as_str()).collect();
        assert!(ids.contains(&"test-one"));
        assert!(ids.contains(&"test-two"));
    }

    #[tokio::test]
    async fn test_load_directory_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "my-skill");
        create_directory_skill(dir, "another-skill");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_load_mixed_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_test_skill(dir, "Single File", "single-file");
        create_directory_skill(dir, "dir-skill");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent_directory() {
        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(Path::new("/nonexistent")).await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_load_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(temp_dir.path()).await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_base_dir_in_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "with-base");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        let skill = skills.iter().find(|s| s.definition.id == "with-base").unwrap();
        assert!(skill.definition.metadata.contains_key("base_dir"));
    }

    #[tokio::test]
    async fn test_load_skill_directly() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "direct-load");

        let loader = FilesystemLoader::new();
        let skill_dir = dir.join("direct-load");
        let skill = loader.load_skill(&skill_dir).await.unwrap();

        assert_eq!(skill.definition.id, "direct-load");
    }
