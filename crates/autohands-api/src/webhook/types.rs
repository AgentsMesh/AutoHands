//! Webhook type definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Webhook event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Webhook ID (path segment).
    pub webhook_id: String,
    /// HTTP method used.
    pub method: String,
    /// Headers (selected).
    pub headers: HashMap<String, String>,
    /// Query parameters.
    pub query: HashMap<String, String>,
    /// Request body (as JSON or raw string).
    pub body: serde_json::Value,
    /// Timestamp.
    pub timestamp: i64,
}

impl WebhookEvent {
    /// Create a new webhook event.
    pub fn new(webhook_id: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            webhook_id: webhook_id.into(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Set the HTTP method.
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into();
        self
    }

    /// Add a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add a query parameter.
    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }

    /// Convert to RunLoop event payload.
    pub fn to_runloop_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "webhook_id": self.webhook_id,
            "method": self.method,
            "headers": self.headers,
            "query": self.query,
            "body": self.body,
            "timestamp": self.timestamp,
        })
    }
}

/// Webhook response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    /// Whether the webhook was processed.
    pub accepted: bool,
    /// Event ID for tracking.
    pub event_id: String,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl WebhookResponse {
    /// Create a successful response.
    pub fn accepted(event_id: impl Into<String>) -> Self {
        Self {
            accepted: true,
            event_id: event_id.into(),
            message: None,
        }
    }

    /// Create a successful response with message.
    pub fn accepted_with_message(event_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            accepted: true,
            event_id: event_id.into(),
            message: Some(message.into()),
        }
    }

    /// Create a rejected response.
    pub fn rejected(event_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            event_id: event_id.into(),
            message: Some(reason.into()),
        }
    }
}

/// Webhook registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRegistration {
    /// Webhook ID.
    pub id: String,
    /// Description.
    pub description: Option<String>,
    /// Target agent to trigger.
    pub agent: Option<String>,
    /// Whether the webhook is enabled.
    pub enabled: bool,
}

impl WebhookRegistration {
    /// Create a new webhook registration.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: None,
            agent: None,
            enabled: true,
        }
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set target agent.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Set enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
