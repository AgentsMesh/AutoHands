
    use super::*;
    use crate::job::JobDefinition;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_job_store() {
        let store = MemoryJobStore::new();
        let def = JobDefinition::new("test-job", "0 * * * *", "agent", "prompt");
        let job = Job::new(def);

        store.save(&job).await.unwrap();

        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().definition.id, "test-job");

        let all = store.load_all().await.unwrap();
        assert_eq!(all.len(), 1);

        store.delete("test-job").await.unwrap();
        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_file_job_store_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        let definition = JobDefinition::new("test-job", "0 * * * *", "test-agent", "Test prompt");
        let job = Job::new(definition);

        store.save(&job).await.unwrap();

        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_some());
        let loaded_job = loaded.unwrap();
        assert_eq!(loaded_job.definition.id, "test-job");
        assert_eq!(loaded_job.definition.agent, "test-agent");
    }

    #[tokio::test]
    async fn test_file_job_store_load_all() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        for i in 0..3 {
            let definition = JobDefinition::new(
                format!("job-{}", i),
                "0 * * * *",
                "agent",
                format!("Prompt {}", i),
            );
            store.save(&Job::new(definition)).await.unwrap();
        }

        let jobs = store.load_all().await.unwrap();
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_file_job_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        let definition = JobDefinition::new("to-delete", "0 * * * *", "agent", "Prompt");
        let job = Job::new(definition);

        store.save(&job).await.unwrap();
        assert!(store.load("to-delete").await.unwrap().is_some());

        store.delete("to-delete").await.unwrap();
        assert!(store.load("to-delete").await.unwrap().is_none());
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(FileJobStore::sanitize_id("simple-job"), "simple-job");
        assert_eq!(
            FileJobStore::sanitize_id("job_with_underscore"),
            "job_with_underscore"
        );
        assert_eq!(
            FileJobStore::sanitize_id("job/with/slashes"),
            "job_with_slashes"
        );
        assert_eq!(
            FileJobStore::sanitize_id("job:with:colons"),
            "job_with_colons"
        );
    }
