
    use super::*;
    use crate::task::TaskPriority;

    #[tokio::test]
    async fn test_queue_enqueue_dequeue() {
        let queue = TaskQueue::new(QueueConfig::default());
        let task = Task::new("test", "general", "payload");

        queue.enqueue(task.clone()).await.unwrap();
        assert_eq!(queue.len().await, 1);

        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, task.id);
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = TaskQueue::new(QueueConfig::default());

        let low = Task::new("low", "general", "").with_priority(TaskPriority::Low);
        let high = Task::new("high", "general", "").with_priority(TaskPriority::High);
        let normal = Task::new("normal", "general", "").with_priority(TaskPriority::Normal);

        queue.enqueue(low).await.unwrap();
        queue.enqueue(high).await.unwrap();
        queue.enqueue(normal).await.unwrap();

        let first = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(first.name, "high");

        let second = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(second.name, "normal");

        let third = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(third.name, "low");
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let config = QueueConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let queue = TaskQueue::new(config);

        queue.enqueue(Task::new("1", "general", "")).await.unwrap();
        queue.enqueue(Task::new("2", "general", "")).await.unwrap();

        let result = queue.enqueue(Task::new("3", "general", "")).await;
        assert!(matches!(result, Err(QueueError::QueueFull)));
    }

    #[tokio::test]
    async fn test_retry() {
        let queue = TaskQueue::new(QueueConfig::default());
        // max_retries = 3 means can retry up to 3 times (retry_count < max_retries)
        let mut task = Task::new("test", "general", "").with_max_retries(3);
        task.status = TaskStatus::Failed;

        // First retry (retry_count becomes 1)
        assert!(queue.retry(task.clone(), "error 1").await.unwrap());
        assert_eq!(queue.len().await, 1);

        // Dequeue and retry again (retry_count becomes 2)
        let task = queue.dequeue().await.unwrap().unwrap();
        assert!(queue.retry(task.clone(), "error 2").await.unwrap());

        // Dequeue and retry again (retry_count becomes 3, which equals max_retries, so DLQ)
        let task = queue.dequeue().await.unwrap().unwrap();
        assert!(!queue.retry(task, "error 3").await.unwrap());
        assert!(queue.is_empty().await);

        let dlq = queue.dead_letter_queue().await;
        assert_eq!(dlq.len(), 1);
    }
