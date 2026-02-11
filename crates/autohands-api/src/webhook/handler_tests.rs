
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

    #[test]
    fn test_verify_github_signature_valid() {
        let secret = "test-secret";
        let body = b"hello world";

        // Compute the expected signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize();
        let hex_sig = hex::encode(result.into_bytes());
        let sig_header = format!("sha256={}", hex_sig);

        assert!(verify_github_signature(secret, &sig_header, body));
    }

    #[test]
    fn test_verify_github_signature_invalid() {
        let secret = "test-secret";
        let body = b"hello world";
        let wrong_sig = "sha256=0000000000000000000000000000000000000000000000000000000000000000";

        assert!(!verify_github_signature(secret, wrong_sig, body));
    }

    #[test]
    fn test_verify_github_signature_bad_prefix() {
        assert!(!verify_github_signature("secret", "sha1=abcdef", b"body"));
    }

    #[test]
    fn test_verify_github_signature_invalid_hex() {
        assert!(!verify_github_signature("secret", "sha256=zzzz", b"body"));
    }

    #[test]
    fn test_verify_github_signature_empty_body() {
        let secret = "my-secret";
        let body = b"";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize();
        let hex_sig = hex::encode(result.into_bytes());
        let sig_header = format!("sha256={}", hex_sig);

        assert!(verify_github_signature(secret, &sig_header, body));
    }

    #[test]
    fn test_verify_github_signature_wrong_secret() {
        let body = b"payload";

        // Sign with one secret
        let mut mac = HmacSha256::new_from_slice(b"correct-secret").unwrap();
        mac.update(body);
        let result = mac.finalize();
        let hex_sig = hex::encode(result.into_bytes());
        let sig_header = format!("sha256={}", hex_sig);

        // Verify with different secret
        assert!(!verify_github_signature("wrong-secret", &sig_header, body));
    }
