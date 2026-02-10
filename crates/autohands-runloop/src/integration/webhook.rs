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
#[path = "webhook_tests.rs"]
mod tests;
