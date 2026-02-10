    use super::*;
    use crate::config::RunLoopConfig;

    #[tokio::test]
    async fn test_timer_once() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::once(
            "test-once",
            Duration::from_millis(100),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert_eq!(timer.id(), "test-once");
        assert!(!timer.is_repeating());
        assert!(timer.is_valid());
        assert_eq!(timer.fire_count(), 1);
    }

    #[tokio::test]
    async fn test_timer_repeating() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::repeating(
            "test-repeat",
            Duration::from_millis(100),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert_eq!(timer.id(), "test-repeat");
        assert!(timer.is_repeating());
        assert!(timer.is_valid());
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::once(
            "test-cancel",
            Duration::from_secs(60),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert!(timer.is_valid());
        timer.cancel();
        assert!(!timer.is_valid());
    }

    #[tokio::test]
    async fn test_timer_builder() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = TimerBuilder::new()
            .id("builder-test")
            .interval(Duration::from_secs(5))
            .repeating()
            .task_type("custom:event")
            .priority(TaskPriority::High)
            .payload(json!({"key": "value"}))
            .build(run_loop);

        assert_eq!(timer.id(), "builder-test");
        assert!(timer.is_repeating());
        assert_eq!(timer.interval(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_timer_builder_default() {
        let builder = TimerBuilder::default();
        assert!(builder.id.is_none());
        assert!(!builder.repeating);
    }

    #[tokio::test]
    async fn test_heartbeat_timer() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = timers::heartbeat(10, run_loop);
        assert_eq!(timer.id(), "heartbeat");
        assert!(timer.is_repeating());
    }

    #[tokio::test]
    async fn test_reminder_timer() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = timers::reminder("my-reminder", Duration::from_secs(300), "Check email", run_loop);
        assert_eq!(timer.id(), "my-reminder");
        assert!(!timer.is_repeating());
    }
