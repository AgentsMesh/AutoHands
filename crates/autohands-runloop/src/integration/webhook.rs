//! Webhook trigger implementation.
//!
//! Provides the `WebhookTrigger` for managing webhook events, and
//! `WebhookInjector` for injecting webhook events via `TaskSubmitter`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::broadcast;
use tracing::info;

// Re-use shared trigger types
use super::trigger_types::{Trigger, TriggerError, TriggerEvent, WebhookConfig};
use autohands_protocols::extension::TaskSubmitter;

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
// WebhookEvent
// ============================================================================

/// Webhook event data.
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

// ============================================================================
// WebhookInjector - Injects webhook events via TaskSubmitter
// ============================================================================

/// Webhook event injector.
///
/// Converts webhook events into tasks via `TaskSubmitter` (which handles
/// both enqueuing and wakeup). Decoupled from RunLoop internals.
pub struct WebhookInjector {
    task_submitter: Arc<dyn TaskSubmitter>,
}

impl WebhookInjector {
    /// Create a new webhook injector.
    pub fn new(task_submitter: Arc<dyn TaskSubmitter>) -> Self {
        Self { task_submitter }
    }

    /// Inject a webhook event as a task.
    pub async fn inject(&self, event: WebhookEvent) -> Result<(), autohands_protocols::error::ExtensionError> {
        self.task_submitter
            .submit_task(
                "trigger:webhook:received",
                json!({
                    "webhook_id": event.webhook_id,
                    "method": event.method,
                    "path": event.path,
                    "body": event.body,
                    "agent": event.agent,
                    "prompt": event.prompt,
                }),
                None,
            )
            .await
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod tests;
