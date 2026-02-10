    use super::*;
    use crate::config::RunLoopConfig;

    #[test]
    fn test_health_status() {
        let healthy = HealthStatus::healthy();
        assert!(healthy.is_healthy);
        assert_eq!(healthy.message, "OK");

        let unhealthy = HealthStatus::unhealthy("error");
        assert!(!unhealthy.is_healthy);
        assert_eq!(unhealthy.message, "error");

        let degraded = HealthStatus::degraded("warning");
        assert!(degraded.is_healthy);
        assert_eq!(degraded.message, "warning");
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let check = LivenessCheck;
        assert_eq!(check.name(), "liveness");

        let status = check.health_check().await.unwrap();
        assert!(status.is_healthy);
    }

    #[test]
    fn test_health_check_observer() {
        let observer = HealthCheckObserver::new(3);
        assert_eq!(observer.check_count(), 0);
        assert_eq!(observer.consecutive_failures(), 0);

        observer.register(Arc::new(LivenessCheck));
        assert_eq!(observer.check_count(), 1);
    }

    #[tokio::test]
    async fn test_task_queue_check() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let check = TaskQueueCheck::new(run_loop.clone(), 1000);

        assert_eq!(check.name(), "task_queue");

        let status = check.health_check().await.unwrap();
        assert!(status.is_healthy);
    }
