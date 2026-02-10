
    use super::*;

    #[test]
    fn test_webhook_event_serialize() {
        let event = WebhookEvent {
            webhook_id: "test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: serde_json::json!({"key": "value"}),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("POST"));
    }

    #[test]
    fn test_webhook_event_new() {
        let event = WebhookEvent::new("test-hook", serde_json::json!({"data": "test"}));
        assert_eq!(event.webhook_id, "test-hook");
        assert_eq!(event.method, "POST");
    }

    #[test]
    fn test_webhook_event_builder() {
        let event = WebhookEvent::new("test", serde_json::json!(null))
            .with_method("PUT")
            .with_header("Authorization", "Bearer token")
            .with_query("key", "value");

        assert_eq!(event.method, "PUT");
        assert_eq!(event.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(event.query.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_response_serialize() {
        let response = WebhookResponse {
            accepted: true,
            event_id: "evt_123".to_string(),
            message: Some("OK".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("accepted"));
        assert!(json.contains("evt_123"));
    }

    #[test]
    fn test_webhook_response_without_message() {
        let response = WebhookResponse {
            accepted: true,
            event_id: "evt_456".to_string(),
            message: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("message"));
    }

    #[test]
    fn test_webhook_response_helpers() {
        let accepted = WebhookResponse::accepted("evt1");
        assert!(accepted.accepted);
        assert!(accepted.message.is_none());

        let with_msg = WebhookResponse::accepted_with_message("evt2", "Done");
        assert!(with_msg.accepted);
        assert_eq!(with_msg.message, Some("Done".to_string()));

        let rejected = WebhookResponse::rejected("evt3", "Invalid");
        assert!(!rejected.accepted);
        assert_eq!(rejected.message, Some("Invalid".to_string()));
    }

    #[test]
    fn test_webhook_registration_serialize() {
        let reg = WebhookRegistration {
            id: "github".to_string(),
            description: Some("GitHub webhook".to_string()),
            agent: Some("deployer".to_string()),
            enabled: true,
        };
        let json = serde_json::to_string(&reg).unwrap();
        assert!(json.contains("github"));
        assert!(json.contains("deployer"));
    }

    #[test]
    fn test_webhook_registration_builder() {
        let reg = WebhookRegistration::new("custom")
            .with_description("Custom webhook")
            .with_agent("handler")
            .with_enabled(false);

        assert_eq!(reg.id, "custom");
        assert_eq!(reg.description, Some("Custom webhook".to_string()));
        assert_eq!(reg.agent, Some("handler".to_string()));
        assert!(!reg.enabled);
    }

    #[test]
    fn test_webhook_event_to_runloop_payload() {
        let event = WebhookEvent::new("test", serde_json::json!({"data": 123}))
            .with_header("X-Custom", "value");

        let payload = event.to_runloop_payload();
        assert_eq!(payload["webhook_id"], "test");
        assert_eq!(payload["body"]["data"], 123);
        assert_eq!(payload["headers"]["X-Custom"], "value");
    }
