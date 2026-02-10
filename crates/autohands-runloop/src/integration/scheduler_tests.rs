    use super::*;

    #[tokio::test]
    async fn test_scheduler_source0_new() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert_eq!(source.id(), "scheduler");
        assert!(!source.is_signaled());
        assert!(source.is_valid());
    }

    #[tokio::test]
    async fn test_scheduler_source0_signal() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert!(!source.is_signaled());
        source.signal_tick();
        assert!(source.is_signaled());
    }

    #[tokio::test]
    async fn test_scheduler_source0_perform() {
        let scheduler = Arc::new(MockScheduler::new());
        scheduler.add_job(MockJob {
            id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test task".to_string(),
        });

        let source = SchedulerSource0::new("scheduler", scheduler);
        source.signal();

        let events = source.perform().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "scheduler:job:due");
    }

    #[tokio::test]
    async fn test_scheduler_source0_cancel() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert!(source.is_valid());
        source.cancel();
        assert!(!source.is_valid());
    }
