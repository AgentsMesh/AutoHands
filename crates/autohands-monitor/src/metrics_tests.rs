
    use super::*;

    #[tokio::test]
    async fn test_registry_counter() {
        let registry = MetricsRegistry::new();
        registry.register_counter("requests_total", "Total requests").await;

        registry.inc_counter("requests_total").await;
        registry.inc_counter("requests_total").await;

        assert_eq!(registry.get_counter("requests_total").await, Some(2));
    }

    #[tokio::test]
    async fn test_registry_gauge() {
        let registry = MetricsRegistry::new();
        registry.register_gauge("active_connections", "Active connections").await;

        registry.set_gauge("active_connections", 5).await;
        assert_eq!(registry.get_gauge("active_connections").await, Some(5));

        registry.set_gauge("active_connections", 3).await;
        assert_eq!(registry.get_gauge("active_connections").await, Some(3));
    }

    #[tokio::test]
    async fn test_export() {
        let registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "A test counter").await;
        registry.inc_counter("test_counter").await;

        let output = registry.export().await;
        assert!(output.contains("# HELP test_counter A test counter"));
        assert!(output.contains("# TYPE test_counter counter"));
        assert!(output.contains("test_counter 1"));
    }
