
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_health_check_result_healthy() {
        let result = HealthCheckResult::healthy();
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult::unhealthy("test error");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.message, Some("test error".to_string()));
    }

    #[test]
    fn test_health_check_result_with_checks() {
        let result = HealthCheckResult::healthy()
            .with_check(ComponentCheck {
                name: "test1".to_string(),
                status: HealthStatus::Healthy,
                details: None,
            })
            .with_check(ComponentCheck {
                name: "test2".to_string(),
                status: HealthStatus::Degraded,
                details: None,
            });

        assert_eq!(result.status, HealthStatus::Degraded);
        assert_eq!(result.checks.len(), 2);
    }

    #[test]
    fn test_unhealthy_overrides_degraded() {
        let result = HealthCheckResult::healthy()
            .with_check(ComponentCheck {
                name: "test1".to_string(),
                status: HealthStatus::Degraded,
                details: None,
            })
            .with_check(ComponentCheck {
                name: "test2".to_string(),
                status: HealthStatus::Unhealthy,
                details: None,
            });

        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_checker() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);

        checker
            .register(Arc::new(LivenessCheck))
            .await;

        let result = checker.check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(checker.check_count(), 1);
        assert_eq!(checker.failure_count(), 0);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let check = LivenessCheck;
        let result = check.check_health().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.name, "liveness");
    }

    #[tokio::test]
    async fn test_memory_check() {
        let check = MemoryCheck::new();
        let result = check.check_health().await;
        assert_eq!(result.name, "memory");
    }

    #[tokio::test]
    async fn test_last_result() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);

        assert!(checker.last_result().await.is_none());

        checker.check().await;

        let result = checker.last_result().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, HealthStatus::Healthy);
    }

    struct FailingCheck;

    impl HealthCheckable for FailingCheck {
        fn name(&self) -> &str {
            "failing"
        }

        fn check_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ComponentCheck> + Send + '_>> {
            Box::pin(async {
                ComponentCheck {
                    name: "failing".to_string(),
                    status: HealthStatus::Unhealthy,
                    details: Some("Always fails".to_string()),
                }
            })
        }
    }

    #[tokio::test]
    async fn test_failing_check_increments_failure_count() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);
        checker.register(Arc::new(FailingCheck)).await;

        let result = checker.check().await;
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(checker.failure_count(), 1);
    }
