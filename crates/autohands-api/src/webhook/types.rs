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
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_serialize() {
        let event = WebhookEvent {
            webhook_id: "test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: serde_json::json!({"key": "value"}),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("POST"));
    }

    #[test]
    fn test_webhook_event_new() {
        let event = WebhookEvent::new("test-hook", serde_json::json!({"data": "test"}));
        assert_eq!(event.webhook_id, "test-hook");
        assert_eq!(event.method, "POST");
    }

    #[test]
    fn test_webhook_event_builder() {
        let event = WebhookEvent::new("test", serde_json::json!(null))
            .with_method("PUT")
            .with_header("Authorization", "Bearer token")
            .with_query("key", "value");

        assert_eq!(event.method, "PUT");
        assert_eq!(event.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(event.query.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_response_serialize() {
        let response = WebhookResponse {
            accepted: true,
            event_id: "evt_123".to_string(),
            message: Some("OK".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("accepted"));
        assert!(json.contains("evt_123"));
    }

    #[test]
    fn test_webhook_response_without_message() {
        let response = WebhookResponse {
            accepted: true,
            event_id: "evt_456".to_string(),
            message: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("message"));
    }

    #[test]
    fn test_webhook_response_helpers() {
        let accepted = WebhookResponse::accepted("evt1");
        assert!(accepted.accepted);
        assert!(accepted.message.is_none());

        let with_msg = WebhookResponse::accepted_with_message("evt2", "Done");
        assert!(with_msg.accepted);
        assert_eq!(with_msg.message, Some("Done".to_string()));

        let rejected = WebhookResponse::rejected("evt3", "Invalid");
        assert!(!rejected.accepted);
        assert_eq!(rejected.message, Some("Invalid".to_string()));
    }

    #[test]
    fn test_webhook_registration_serialize() {
        let reg = WebhookRegistration {
            id: "github".to_string(),
            description: Some("GitHub webhook".to_string()),
            agent: Some("deployer".to_string()),
            enabled: true,
        };
        let json = serde_json::to_string(&reg).unwrap();
        assert!(json.contains("github"));
        assert!(json.contains("deployer"));
    }

    #[test]
    fn test_webhook_registration_builder() {
        let reg = WebhookRegistration::new("custom")
            .with_description("Custom webhook")
            .with_agent("handler")
            .with_enabled(false);

        assert_eq!(reg.id, "custom");
        assert_eq!(reg.description, Some("Custom webhook".to_string()));
        assert_eq!(reg.agent, Some("handler".to_string()));
        assert!(!reg.enabled);
    }

    #[test]
    fn test_webhook_event_to_runloop_payload() {
        let event = WebhookEvent::new("test", serde_json::json!({"data": 123}))
            .with_header("X-Custom", "value");

        let payload = event.to_runloop_payload();
        assert_eq!(payload["webhook_id"], "test");
        assert_eq!(payload["body"]["data"], 123);
        assert_eq!(payload["headers"]["X-Custom"], "value");
    }
}
