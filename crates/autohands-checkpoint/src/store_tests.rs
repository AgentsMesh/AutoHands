
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_checkpoint_store_save_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let checkpoint = Checkpoint::new(
            "session1",
            5,
            serde_json::json!([{"role": "user", "content": "hello"}]),
            serde_json::json!({"key": "value"}),
        );
        let cp_id = checkpoint.id;

        // Save
        store.save(&checkpoint).await.unwrap();

        // Get by ID
        let loaded = store.get(&cp_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_cp = loaded.unwrap();
        assert_eq!(loaded_cp.session_id, "session1");
        assert_eq!(loaded_cp.turn, 5);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_get_latest() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create multiple checkpoints for same session
        for turn in [1, 5, 3, 10, 7] {
            let cp = Checkpoint::new(
                "session1",
                turn,
                serde_json::json!([]),
                serde_json::json!({}),
            );
            store.save(&cp).await.unwrap();
        }

        // Get latest should return turn 10
        let latest = store.get_latest("session1").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().turn, 10);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_list() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create checkpoints in random order
        for turn in [5, 1, 3] {
            let cp = Checkpoint::new("session1", turn, serde_json::json!([]), serde_json::json!({}));
            store.save(&cp).await.unwrap();
        }

        // List should return sorted by turn
        let list = store.list("session1").await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].turn, 1);
        assert_eq!(list[1].turn, 3);
        assert_eq!(list[2].turn, 5);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let checkpoint = Checkpoint::new("session1", 5, serde_json::json!([]), serde_json::json!({}));
        let cp_id = checkpoint.id;

        store.save(&checkpoint).await.unwrap();
        assert!(store.get(&cp_id).await.unwrap().is_some());

        store.delete(&cp_id).await.unwrap();
        assert!(store.get(&cp_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create checkpoints for two sessions
        for turn in 1..=3 {
            let cp1 = Checkpoint::new("session1", turn, serde_json::json!([]), serde_json::json!({}));
            let cp2 = Checkpoint::new("session2", turn, serde_json::json!([]), serde_json::json!({}));
            store.save(&cp1).await.unwrap();
            store.save(&cp2).await.unwrap();
        }

        assert_eq!(store.list("session1").await.unwrap().len(), 3);
        assert_eq!(store.list("session2").await.unwrap().len(), 3);

        // Delete session1
        store.delete_session("session1").await.unwrap();

        assert_eq!(store.list("session1").await.unwrap().len(), 0);
        assert_eq!(store.list("session2").await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_get_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let result = store.get(&Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_filename() {
        let uuid = Uuid::new_v4();
        let filename = format!("{}_turn_000005.json", uuid);
        let parsed = FileCheckpointStore::parse_filename(&filename);
        assert!(parsed.is_some());
        let (parsed_uuid, parsed_turn) = parsed.unwrap();
        assert_eq!(parsed_uuid, uuid);
        assert_eq!(parsed_turn, 5);
    }

    #[test]
    fn test_sanitize_session_id() {
        assert_eq!(FileCheckpointStore::sanitize_session_id("simple-session"), "simple-session");
        assert_eq!(FileCheckpointStore::sanitize_session_id("session/with/slashes"), "session_with_slashes");
        assert_eq!(FileCheckpointStore::sanitize_session_id("session:with:colons"), "session_with_colons");
    }
