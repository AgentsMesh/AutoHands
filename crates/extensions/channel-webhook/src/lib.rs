//! # AutoHands Channel - Webhook
//!
//! Webhook channel for outbound notifications.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use autohands_protocols::channel::{
    Channel, ChannelCapabilities, ChannelId, InboundMessage, OutboundMessage, ReplyAddress,
    SentMessage,
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
    pub target: ReplyAddress,
    pub content: String,
}

/// Webhook channel implementation.
pub struct WebhookChannel {
    id: ChannelId,
    config: WebhookConfig,
    client: Client,
    capabilities: ChannelCapabilities,
    message_tx: broadcast::Sender<InboundMessage>,
    started: AtomicBool,
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
            started: AtomicBool::new(false),
        }
    }

    /// Check if the channel is started.
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::SeqCst)
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
    fn id(&self) -> &ChannelId {
        &self.id
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        &self.capabilities
    }

    async fn start(&self) -> Result<(), ChannelError> {
        self.started.store(true, Ordering::SeqCst);
        debug!("Webhook channel started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), ChannelError> {
        self.started.store(false, Ordering::SeqCst);
        debug!("Webhook channel stopped");
        Ok(())
    }

    async fn send(
        &self,
        target: &ReplyAddress,
        message: OutboundMessage,
    ) -> Result<SentMessage, ChannelError> {
        if !self.is_started() {
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

    fn inbound(&self) -> broadcast::Receiver<InboundMessage> {
        self.message_tx.subscribe()
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod tests;
