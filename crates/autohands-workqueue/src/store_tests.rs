
    use super::*;
    use crate::task::TaskPriority;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_task_store_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTaskStore::new(temp_dir.path()).await.unwrap();

        let task = Task::new("test-task", "test-agent", "Test payload");
        let task_id = task.id;

        // Save
        store.save(&task).await.unwrap();

        // Load
        let loaded = store.load(&task_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_task = loaded.unwrap();
        assert_eq!(loaded_task.name, "test-task");
        assert_eq!(loaded_task.agent, "test-agent");
    }

    #[tokio::test]
    async fn test_file_task_store_load_pending() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTaskStore::new(temp_dir.path()).await.unwrap();

        // Save multiple pending tasks with different priorities
        let high_task = Task::new("high", "agent", "High priority")
            .with_priority(TaskPriority::High);
        let low_task = Task::new("low", "agent", "Low priority")
            .with_priority(TaskPriority::Low);
        let normal_task = Task::new("normal", "agent", "Normal priority");

        store.save(&low_task).await.unwrap();
        store.save(&normal_task).await.unwrap();
        store.save(&high_task).await.unwrap();

        // Load pending - should be sorted by priority
        let pending = store.load_pending().await.unwrap();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].name, "high");
        assert_eq!(pending[1].name, "normal");
        assert_eq!(pending[2].name, "low");
    }

    #[tokio::test]
    async fn test_file_task_store_status_change() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTaskStore::new(temp_dir.path()).await.unwrap();

        let mut task = Task::new("test", "agent", "payload");
        let task_id = task.id;

        // Save as pending
        store.save(&task).await.unwrap();
        assert!(store.task_path(&task_id, TaskStatus::Pending).exists());

        // Change to running
        task.status = TaskStatus::Running;
        store.save(&task).await.unwrap();
        assert!(!store.task_path(&task_id, TaskStatus::Pending).exists());
        assert!(store.task_path(&task_id, TaskStatus::Running).exists());

        // Change to completed
        task.status = TaskStatus::Completed;
        store.save(&task).await.unwrap();
        assert!(!store.task_path(&task_id, TaskStatus::Running).exists());
        assert!(store.task_path(&task_id, TaskStatus::Completed).exists());
    }

    #[tokio::test]
    async fn test_file_task_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTaskStore::new(temp_dir.path()).await.unwrap();

        let task = Task::new("to-delete", "agent", "payload");
        let task_id = task.id;

        store.save(&task).await.unwrap();
        assert!(store.load(&task_id).await.unwrap().is_some());

        store.delete(&task_id).await.unwrap();
        assert!(store.load(&task_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_task_store_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTaskStore::new(temp_dir.path()).await.unwrap();

        let result = store.load(&Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }
