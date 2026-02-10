
    use super::*;

    #[test]
    fn test_webhook_response_creation() {
        let resp = WebhookResponse::accepted("test-event");
        assert!(resp.accepted);
        assert_eq!(resp.event_id, "test-event");
    }

    #[test]
    fn test_webhook_event_creation() {
        let event = WebhookEvent::new("test", serde_json::json!({"data": "value"}));
        assert_eq!(event.webhook_id, "test");
        assert_eq!(event.method, "POST");
    }

    #[test]
    fn test_webhook_event_with_headers() {
        let event = WebhookEvent::new("test", serde_json::json!(null))
            .with_header("X-Custom", "value")
            .with_header("Authorization", "Bearer token");

        assert_eq!(event.headers.len(), 2);
        assert_eq!(event.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_registration() {
        let reg = WebhookRegistration::new("test-hook")
            .with_description("Test webhook")
            .with_agent("test-agent")
            .with_enabled(true);

        assert_eq!(reg.id, "test-hook");
        assert_eq!(reg.description, Some("Test webhook".to_string()));
        assert_eq!(reg.agent, Some("test-agent".to_string()));
        assert!(reg.enabled);
    }
