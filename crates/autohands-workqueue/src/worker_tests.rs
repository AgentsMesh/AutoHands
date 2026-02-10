
    use super::*;

    struct TestHandler;

    #[async_trait]
    impl TaskHandler for TestHandler {
        async fn handle(&self, _task: &Task) -> Result<(), QueueError> {
            Ok(())
        }
    }

    #[test]
    fn test_worker_new() {
        let worker = Worker::new(1);
        assert_eq!(worker.id(), 1);
        assert!(!worker.is_running());
        assert_eq!(worker.tasks_completed(), 0);
    }

    #[tokio::test]
    async fn test_worker_process_success() {
        let worker = Worker::new(1);
        let task = Task::new("test", "general", "payload");
        let queue = TaskQueue::new(QueueConfig::default());
        let handler = TestHandler;

        worker.process(task, &handler, &queue).await.unwrap();
        assert_eq!(worker.tasks_completed(), 1);
        assert_eq!(worker.tasks_failed(), 0);
    }

    #[test]
    fn test_worker_pool_new() {
        let config = QueueConfig {
            max_workers: 4,
            ..Default::default()
        };
        let pool = WorkerPool::new(config);

        assert!(!pool.is_running());
        assert_eq!(pool.available_workers(), 4);
    }

    #[tokio::test]
    async fn test_worker_pool_submit() {
        let pool = WorkerPool::new(QueueConfig::default());
        pool.start();

        let task = Task::new("test", "general", "payload");
        let handler = Arc::new(TestHandler);
        let queue = Arc::new(TaskQueue::new(QueueConfig::default()));

        pool.submit(task, handler, queue).await.unwrap();

        // Wait for task to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        pool.stop();
    }
