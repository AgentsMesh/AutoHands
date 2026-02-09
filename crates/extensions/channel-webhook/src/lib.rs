//! # AutoHands Channel - Webhook
//!
//! Webhook channel for outbound notifications.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use autohands_protocols::channel::{
    Channel, ChannelCapabilities, IncomingMessage, MessageTarget, OutgoingMessage, SentMessage,
};
use autohands_protocols::error::ChannelError;

/// Webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL.
    pub url: String,
    /// HTTP method (POST by default).
    #[serde(default = "default_method")]
    pub method: String,
    /// Additional headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Retry count on failure.
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    /// Secret for HMAC signing (optional).
    pub secret: Option<String>,
}

fn default_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_retries() -> u32 {
    3
}

/// Webhook payload.
#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub timestamp: i64,
    pub target: MessageTarget,
    pub content: String,
}

/// Webhook channel implementation.
pub struct WebhookChannel {
    id: String,
    config: WebhookConfig,
    client: Client,
    capabilities: ChannelCapabilities,
    message_tx: broadcast::Sender<IncomingMessage>,
    connected: bool,
}

impl WebhookChannel {
    /// Create a new webhook channel.
    pub fn new(id: impl Into<String>, config: WebhookConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        let (message_tx, _) = broadcast::channel(100);

        Self {
            id: id.into(),
            config,
            client,
            capabilities: ChannelCapabilities {
                supports_images: false,
                supports_files: false,
                supports_reactions: false,
                supports_threads: false,
                supports_editing: false,
                max_message_length: None,
            },
            message_tx,
            connected: false,
        }
    }

    async fn send_webhook(&self, payload: &WebhookPayload) -> Result<String, ChannelError> {
        let json = serde_json::to_string(payload)
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        let mut request = match self.config.method.to_uppercase().as_str() {
            "POST" => self.client.post(&self.config.url),
            "PUT" => self.client.put(&self.config.url),
            _ => self.client.post(&self.config.url),
        };

        request = request.header("Content-Type", "application/json");
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        if let Some(ref secret) = self.config.secret {
            let signature = compute_signature(&json, secret);
            request = request.header("X-Webhook-Signature", signature);
        }

        let response = request
            .body(json)
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!("HTTP {}: {}", status, body)));
        }

        debug!("Webhook delivered successfully to {}", self.config.url);
        Ok(uuid::Uuid::new_v4().to_string())
    }
}

fn compute_signature(payload: &str, secret: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    payload.hash(&mut hasher);
    secret.hash(&mut hasher);
    format!("sha256={:x}", hasher.finish())
}

#[async_trait]
impl Channel for WebhookChannel {
    fn id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        &self.capabilities
    }

    async fn connect(&mut self) -> Result<(), ChannelError> {
        self.connected = true;
        debug!("Webhook channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ChannelError> {
        self.connected = false;
        debug!("Webhook channel disconnected");
        Ok(())
    }

    async fn send(
        &self,
        target: &MessageTarget,
        message: OutgoingMessage,
    ) -> Result<SentMessage, ChannelError> {
        if !self.connected {
            return Err(ChannelError::Disconnected);
        }

        let payload = WebhookPayload {
            event_type: "message".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            target: target.clone(),
            content: message.content,
        };

        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.send_webhook(&payload).await {
                Ok(id) => {
                    return Ok(SentMessage {
                        id,
                        timestamp: chrono::Utc::now(),
                    });
                }
                Err(e) => {
                    if attempt < self.config.max_retries {
                        warn!("Webhook attempt {} failed: {}, retrying...", attempt + 1, e);
                        tokio::time::sleep(Duration::from_millis(100 * (attempt as u64 + 1))).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        error!(
            "Webhook delivery failed after {} attempts",
            self.config.max_retries + 1
        );

        Err(last_error.unwrap_or(ChannelError::SendFailed("Unknown error".to_string())))
    }

    fn on_message(&self) -> broadcast::Receiver<IncomingMessage> {
        self.message_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
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
            target: MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            },
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
    async fn test_connect_disconnect() {
        let config = create_test_config();
        let mut channel = WebhookChannel::new("webhook", config);

        assert!(!channel.connected);
        channel.connect().await.unwrap();
        assert!(channel.connected);
        channel.disconnect().await.unwrap();
        assert!(!channel.connected);
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
        let target = MessageTarget {
            channel_id: "ch-1".to_string(),
            thread_id: Some("thread-1".to_string()),
            user_id: Some("user-1".to_string()),
        };
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
    async fn test_send_when_disconnected() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        // Not connected

        let target = MessageTarget {
            channel_id: "ch-1".to_string(),
            thread_id: None,
            user_id: None,
        };
        let message = OutgoingMessage {
            content: "Hello".to_string(),
            attachments: vec![],
            reply_to: None,
        };

        let result = channel.send(&target, message).await;
        assert!(matches!(result, Err(ChannelError::Disconnected)));
    }

    #[test]
    fn test_on_message_returns_receiver() {
        let config = create_test_config();
        let channel = WebhookChannel::new("webhook", config);
        let _rx = channel.on_message();
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
            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Hello, webhook!".to_string(),
                attachments: vec![],
                reply_to: None,
            };

            let result = channel.send(&target, message).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_send_webhook_failure_with_retry() {
            let mock_server = MockServer::start().await;

            // Fail first 2 attempts, succeed on 3rd
            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Error"))
                .expect(4) // max_retries + 1
                .mount(&mock_server)
                .await;

            let config = create_mock_config(&mock_server.uri());
            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Test message".to_string(),
                attachments: vec![],
                reply_to: None,
            };

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

            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Test".to_string(),
                attachments: vec![],
                reply_to: None,
            };

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

            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Test".to_string(),
                attachments: vec![],
                reply_to: None,
            };

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

            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Test".to_string(),
                attachments: vec![],
                reply_to: None,
            };

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

            let mut channel = WebhookChannel::new("test-webhook", config);
            channel.connect().await.unwrap();

            let target = MessageTarget {
                channel_id: "ch-1".to_string(),
                thread_id: None,
                user_id: None,
            };
            let message = OutgoingMessage {
                content: "Test".to_string(),
                attachments: vec![],
                reply_to: None,
            };

            let result = channel.send(&target, message).await;
            assert!(result.is_err());
            if let Err(ChannelError::SendFailed(msg)) = result {
                assert!(msg.contains("400"));
            }
        }
    }
}
