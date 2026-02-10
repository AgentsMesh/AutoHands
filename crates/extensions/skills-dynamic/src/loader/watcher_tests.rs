    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(path: &PathBuf) {
        let content = r#"---
id: test-watch
name: Watch Test
description: Test skill for watching
---

Test content.
"#;
        fs::write(path, content).unwrap();
    }

    #[tokio::test]
    async fn test_watcher_creation() {
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        assert!(watcher.watched_paths.is_empty());
    }

    #[tokio::test]
    async fn test_watch_path() {
        let temp_dir = TempDir::new().unwrap();
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let mut watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        watcher.watch(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(watcher.watched_paths.len(), 1);
    }

    #[tokio::test]
    async fn test_watch_nonexistent_path() {
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let mut watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        let result = watcher.watch(PathBuf::from("/nonexistent/path"));

        // Should not error, just skip
        assert!(result.is_ok());
        assert!(watcher.watched_paths.is_empty());
    }

    #[test]
    fn test_is_relevant_event() {
        use notify::event::{CreateKind, ModifyKind, RemoveKind};

        // Create event for markdown file - should be relevant
        let create_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("/test/skill.markdown")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&create_event));

        // Modify event for SKILL.md - should be relevant
        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![PathBuf::from("/test/my-skill/SKILL.md")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&modify_event));

        // Remove event - should be relevant
        let remove_event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![PathBuf::from("/test/old.markdown")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&remove_event));

        // Non-skill file - should not be relevant
        let other_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("/test/file.txt")],
            attrs: Default::default(),
        };
        assert!(!SkillWatcher::is_relevant_event(&other_event));
    }

    #[tokio::test]
    async fn test_reload_skills() {
        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("reload-test.markdown");
        create_test_skill(&skill_path);

        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));
        let watched_paths = vec![temp_dir.path().to_path_buf()];

        SkillWatcher::reload_skills(&skills, &fs_loader, &available_tools, &watched_paths)
            .await
            .unwrap();

        let skills_guard = skills.read().await;
        assert_eq!(skills_guard.len(), 1);
        assert!(skills_guard.contains_key("test-watch"));
    }
