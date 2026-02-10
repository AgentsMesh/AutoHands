    use super::*;

    fn test_config() -> WebhookConfig {
        WebhookConfig {
            id: "test-webhook".to_string(),
            path: "/webhook/test".to_string(),
            agent: "general".to_string(),
            prompt_template: None,
            enabled: true,
            secret: None,
        }
    }

    #[test]
    fn test_webhook_trigger_new() {
        let trigger = WebhookTrigger::new(test_config());
        assert_eq!(trigger.id(), "test-webhook");
        assert_eq!(trigger.trigger_type(), "webhook");
        assert!(trigger.is_enabled());
    }

    #[test]
    fn test_webhook_fire() {
        let trigger = WebhookTrigger::new(test_config());
        let event = trigger.fire(json!({"test": true})).unwrap();

        assert_eq!(event.trigger_id, "test-webhook");
        assert_eq!(event.data["test"], true);
    }

    #[test]
    fn test_webhook_fire_disabled() {
        let mut config = test_config();
        config.enabled = false;
        let trigger = WebhookTrigger::new(config);

        let result = trigger.fire(json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_secret() {
        let mut config = test_config();
        config.secret = Some("secret123".to_string());
        let trigger = WebhookTrigger::new(config);

        assert!(trigger.verify_secret(Some("secret123")));
        assert!(!trigger.verify_secret(Some("wrong")));
        assert!(!trigger.verify_secret(None));
    }

    #[test]
    fn test_verify_secret_no_secret() {
        let trigger = WebhookTrigger::new(test_config());
        assert!(trigger.verify_secret(None));
        assert!(trigger.verify_secret(Some("anything")));
    }

    // Source1 tests
    #[tokio::test]
    async fn test_webhook_source1() {
        let source = WebhookSource1::new("webhook");
        assert_eq!(source.id(), "webhook");
        assert!(source.is_valid());
    }

    #[tokio::test]
    async fn test_webhook_source1_handle() {
        let source = WebhookSource1::new("webhook");

        let msg = WebhookSource1::create_message(WebhookEvent {
            webhook_id: "hook-1".to_string(),
            method: "POST".to_string(),
            path: "/api/webhook".to_string(),
            body: json!({"key": "value"}),
            agent: Some("general".to_string()),
            prompt: None,
        });

        let events = source.handle(msg).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "trigger:webhook:received");
    }
