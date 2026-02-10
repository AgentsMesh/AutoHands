    use super::*;
    use crate::config::RunLoopConfig;

    #[tokio::test]
    async fn test_cron_timer_creation() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "test-cron",
            "0 * * * * *", // Every minute
            || Task::new("test:cron", serde_json::Value::Null),
            run_loop.clone(),
        )
        .expect("Valid cron expression");

        assert_eq!(timer.id(), "test-cron");
        assert_eq!(timer.cron_expr(), "0 * * * * *");
        assert!(timer.is_valid());
        assert_eq!(timer.fire_count(), 1); // First schedule
    }

    #[tokio::test]
    async fn test_cron_timer_invalid_expr() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let result = CronTimer::new(
            "bad-cron",
            "invalid cron expression",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cron_timer_cancel() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "cancel-test",
            "0 * * * * *",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        )
        .unwrap();

        assert!(timer.is_valid());
        timer.cancel();
        assert!(!timer.is_valid());
    }

    #[tokio::test]
    async fn test_cron_timer_builder() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimerBuilder::new("0 */5 * * * *")
            .id("builder-test")
            .task_type("custom:task")
            .priority(TaskPriority::High)
            .payload(json!({"key": "value"}))
            .build(run_loop)
            .unwrap();

        assert_eq!(timer.id(), "builder-test");
        assert_eq!(timer.cron_expr(), "0 */5 * * * *");
    }

    #[tokio::test]
    async fn test_cron_timer_next_fire_time() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "next-fire",
            "0 * * * * *",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        )
        .unwrap();

        let next = timer.next_fire_time();
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn test_schedule_presets() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = schedules::every_seconds("every-5s", 5, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::every_minutes("every-10m", 10, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::every_hours("every-2h", 2, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::daily_at("daily-9am", 9, 0, run_loop.clone()).unwrap();
        assert!(timer.is_valid());
    }
