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

    #[test]
    fn test_webhook_event_creation() {
        let event = WebhookEvent {
            webhook_id: "hook-1".to_string(),
            method: "POST".to_string(),
            path: "/api/webhook".to_string(),
            body: json!({"key": "value"}),
            agent: Some("general".to_string()),
            prompt: None,
        };

        assert_eq!(event.webhook_id, "hook-1");
        assert_eq!(event.method, "POST");
        assert_eq!(event.body["key"], "value");
    }

    #[test]
    fn test_webhook_injector_creation() {
        use crate::RunLoopConfig;
        // RunLoop implements TaskSubmitter
        let run_loop: Arc<dyn TaskSubmitter> = Arc::new(crate::RunLoop::new(RunLoopConfig::default()));
        let injector = WebhookInjector::new(run_loop);
        // Injector created successfully - no panics
        let _ = injector;
    }
