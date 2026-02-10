    use super::*;

    fn create_test_config() -> WebhookConfig {
        WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            timeout_seconds: 30,
            max_retries: 3,
            secret: None,
        }
    }

    fn create_test_target() -> ReplyAddress {
        ReplyAddress::new("webhook", "target-1")
    }

    #[test]
    fn test_webhook_config_defaults() {
        let json = serde_json::json!({
            "url": "https://example.com/webhook"
        });
        let config: WebhookConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.method, "POST");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_webhook_channel_creation() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        assert_eq!(channel.id(), "webhook");
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event_type: "message".to_string(),
            timestamp: 1234567890,
            target: ReplyAddress::new("ch-1", "user-1"),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_compute_signature() {
        let sig1 = compute_signature("payload", "secret");
        let sig2 = compute_signature("payload", "secret");
        assert_eq!(sig1, sig2);

        let sig3 = compute_signature("different", "secret");
        assert_ne!(sig1, sig3);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);

        assert!(!channel.is_started());
        channel.start().await.unwrap();
        assert!(channel.is_started());
        channel.stop().await.unwrap();
        assert!(!channel.is_started());
    }

    #[test]
    fn test_capabilities() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        let caps = channel.capabilities();
        assert!(!caps.supports_images);
        assert!(!caps.supports_files);
    }

    #[test]
    fn test_default_method() {
        assert_eq!(default_method(), "POST");
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_default_retries() {
        assert_eq!(default_retries(), 3);
    }

    #[test]
    fn test_webhook_config_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("X-Custom".to_string(), "value".to_string());

        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers,
            timeout_seconds: 60,
            max_retries: 5,
            secret: Some("my-secret".to_string()),
        };

        assert_eq!(config.headers.len(), 2);
        assert!(config.secret.is_some());
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_webhook_config_serialization() {
        let config = create_test_config();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("https://example.com/webhook"));
        assert!(json.contains("POST"));
    }

    #[test]
    fn test_webhook_config_deserialization_full() {
        let json = serde_json::json!({
            "url": "https://example.com/hook",
            "method": "PUT",
            "headers": {"X-API-Key": "key123"},
            "timeout_seconds": 45,
            "max_retries": 2,
            "secret": "secret123"
        });
        let config: WebhookConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.method, "PUT");
        assert_eq!(config.timeout_seconds, 45);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.secret, Some("secret123".to_string()));
        assert!(config.headers.contains_key("X-API-Key"));
    }

    #[test]
    fn test_webhook_payload_fields() {
        let target = ReplyAddress::with_thread("ch-1", "user-1", "thread-1");
        let payload = WebhookPayload {
            event_type: "notification".to_string(),
            timestamp: 1700000000,
            target,
            content: "Test content".to_string(),
        };
        assert_eq!(payload.event_type, "notification");
        assert_eq!(payload.timestamp, 1700000000);
        assert_eq!(payload.content, "Test content");
    }

    #[test]
    fn test_compute_signature_with_different_secrets() {
        let sig1 = compute_signature("payload", "secret1");
        let sig2 = compute_signature("payload", "secret2");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_compute_signature_format() {
        let sig = compute_signature("test", "secret");
        assert!(sig.starts_with("sha256="));
    }

    #[test]
    fn test_channel_capabilities_full() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        let caps = channel.capabilities();
        assert!(!caps.supports_reactions);
        assert!(!caps.supports_threads);
        assert!(!caps.supports_editing);
        assert!(caps.max_message_length.is_none());
    }

    #[tokio::test]
    async fn test_send_when_not_started() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        // Not started

        let target = create_test_target();
        let message = OutboundMessage::text("Hello");

        let result = channel.send(&target, message).await;
        assert!(matches!(result, Err(ChannelError::Disconnected)));
    }

    #[test]
    fn test_inbound_returns_receiver() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        let _rx = channel.inbound();
        // Should not panic
    }

    #[test]
    fn test_webhook_config_clone() {
        let config = create_test_config();
        let cloned = config.clone();
        assert_eq!(cloned.url, config.url);
        assert_eq!(cloned.method, config.method);
    }

    #[test]
    fn test_webhook_config_debug() {
        let config = create_test_config();
        let debug = format!("{:?}", config);
        assert!(debug.contains("WebhookConfig"));
    }

    // Wiremock-based tests for HTTP webhook functionality
    mod http_tests {
        use super::*;
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        fn create_mock_config(url: &str) -> WebhookConfig {
            WebhookConfig {
                url: url.to_string(),
                method: "POST".to_string(),
                headers: HashMap::new(),
                timeout_seconds: 30,
                max_retries: 3,
                secret: None,
            }
        }

        #[tokio::test]
        async fn test_send_webhook_success() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&mock_server)
                .await;

            let config = create_mock_config(&mock_server.uri());
            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Hello, webhook!");

            let result = channel.send(&target, message).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_send_webhook_failure_with_retry() {
            let mock_server = MockServer::start().await;

            // Fail all attempts
            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Error"))
                .expect(4) // max_retries + 1
                .mount(&mock_server)
                .await;

            let config = create_mock_config(&mock_server.uri());
            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Test message");

            let result = channel.send(&target, message).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_send_webhook_with_put_method() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("PUT"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&mock_server)
                .await;

            let mut config = create_mock_config(&mock_server.uri());
            config.method = "PUT".to_string();

            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Test");

            let result = channel.send(&target, message).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_send_webhook_with_custom_headers() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .and(matchers::header("X-Custom-Header", "custom-value"))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&mock_server)
                .await;

            let mut config = create_mock_config(&mock_server.uri());
            config.headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Test");

            let result = channel.send(&target, message).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_send_webhook_with_signature() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .and(matchers::header_exists("X-Webhook-Signature"))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&mock_server)
                .await;

            let mut config = create_mock_config(&mock_server.uri());
            config.secret = Some("my-secret-key".to_string());

            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Test");

            let result = channel.send(&target, message).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_send_webhook_4xx_error() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))
                .mount(&mock_server)
                .await;

            let mut config = create_mock_config(&mock_server.uri());
            config.max_retries = 0; // No retries

            let channel = WebhookChannel::new("test-webhook", config);
            channel.start().await.unwrap();

            let target = ReplyAddress::new("ch-1", "user-1");
            let message = OutboundMessage::text("Test");

            let result = channel.send(&target, message).await;
            assert!(result.is_err());
            if let Err(ChannelError::SendFailed(msg)) = result {
                assert!(msg.contains("400"));
            }
        }
    }
