//! Webhook trigger implementation.
//!
//! Provides both the full WebhookTrigger implementation and
//! the Source1 adapter for RunLoop integration.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

// Re-use shared trigger types
use super::trigger_types::{Trigger, TriggerError, TriggerEvent, WebhookConfig};
use crate::error::RunLoopResult;
use crate::task::{Task, TaskPriority, TaskSource};
use crate::mode::RunLoopMode;
use crate::source::{PortMessage, Source1, Source1Receiver};

// ============================================================================
// WebhookTrigger - Full implementation
// ============================================================================

/// Webhook trigger.
pub struct WebhookTrigger {
    config: WebhookConfig,
    enabled: AtomicBool,
    event_sender: broadcast::Sender<TriggerEvent>,
}

impl WebhookTrigger {
    /// Create a new webhook trigger.
    pub fn new(config: WebhookConfig) -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            enabled: AtomicBool::new(config.enabled),
            config,
            event_sender: sender,
        }
    }

    /// Subscribe to trigger events.
    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.event_sender.subscribe()
    }

    /// Fire the trigger with payload.
    pub fn fire(&self, payload: serde_json::Value) -> Result<TriggerEvent, TriggerError> {
        if !self.is_enabled() {
            return Err(TriggerError::Disabled(self.config.id.clone()));
        }

        let prompt = self
            .config
            .prompt_template
            .as_ref()
            .map(|t| t.clone())
            .unwrap_or_else(|| "Process webhook event".to_string());

        let event = TriggerEvent::new(&self.config.id, "webhook", &self.config.agent, prompt)
            .with_data(payload);

        let _ = self.event_sender.send(event.clone());
        info!("Webhook trigger fired: {}", self.config.id);

        Ok(event)
    }

    /// Get the webhook path.
    pub fn path(&self) -> &str {
        &self.config.path
    }

    /// Verify a webhook secret.
    pub fn verify_secret(&self, provided: Option<&str>) -> bool {
        match (&self.config.secret, provided) {
            (Some(expected), Some(provided)) => expected == provided,
            (None, _) => true,
            (Some(_), None) => false,
        }
    }
}

#[async_trait]
impl Trigger for WebhookTrigger {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn trigger_type(&self) -> &str {
        "webhook"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    async fn start(&self) -> Result<(), TriggerError> {
        self.enabled.store(true, Ordering::SeqCst);
        info!("Webhook trigger started: {}", self.config.id);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerError> {
        self.enabled.store(false, Ordering::SeqCst);
        info!("Webhook trigger stopped: {}", self.config.id);
        Ok(())
    }
}

// ============================================================================
// WebhookSource1 - RunLoop Source1 adapter
// ============================================================================

/// Webhook event for Source1.
#[derive(Debug, Clone)]
pub struct WebhookEvent {
    /// Webhook ID.
    pub webhook_id: String,
    /// Request method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Request body.
    pub body: serde_json::Value,
    /// Agent to handle the webhook.
    pub agent: Option<String>,
    /// Prompt for the agent.
    pub prompt: Option<String>,
}

/// Webhook Source1.
///
/// Receives webhook events and produces RunLoop events.
pub struct WebhookSource1 {
    id: String,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl WebhookSource1 {
    /// Create a new webhook source.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Default],
        }
    }

    /// Create with custom modes.
    pub fn with_modes(mut self, modes: Vec<RunLoopMode>) -> Self {
        self.modes = modes;
        self
    }

    /// Create a Source1Receiver for this source.
    pub fn create_receiver(self) -> (Source1Receiver, mpsc::Sender<PortMessage>) {
        let (tx, rx) = mpsc::channel(256);
        let source = Arc::new(self);
        (Source1Receiver::new(source, rx), tx)
    }

    /// Create a PortMessage from a WebhookEvent.
    pub fn create_message(event: WebhookEvent) -> PortMessage {
        PortMessage::new(
            "webhook",
            json!({
                "webhook_id": event.webhook_id,
                "method": event.method,
                "path": event.path,
                "body": event.body,
                "agent": event.agent,
                "prompt": event.prompt,
            }),
        )
    }
}

#[async_trait]
impl Source1 for WebhookSource1 {
    fn id(&self) -> &str {
        &self.id
    }

    async fn handle(&self, msg: PortMessage) -> RunLoopResult<Vec<Task>> {
        let webhook_id = msg.payload["webhook_id"].as_str().unwrap_or("");
        let method = msg.payload["method"].as_str().unwrap_or("POST");
        let path = msg.payload["path"].as_str().unwrap_or("/");
        let body = msg.payload["body"].clone();
        let agent = msg.payload["agent"].as_str();
        let prompt = msg.payload["prompt"].as_str();

        debug!("Webhook received: {} {} {}", webhook_id, method, path);

        let event = Task::new(
            "trigger:webhook:received",
            json!({
                "webhook_id": webhook_id,
                "method": method,
                "path": path,
                "body": body,
                "agent": agent,
                "prompt": prompt,
            }),
        )
        .with_source(TaskSource::Webhook)
        .with_priority(TaskPriority::Normal);

        Ok(vec![event])
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
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
}
