//! Notification types shared between channels.

use serde::{Deserialize, Serialize};

/// Supported notification channels.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifyChannel {
    /// Log to console/file (always available).
    Log,
    /// Send email.
    Email,
    /// Send to Slack channel or user.
    Slack,
    /// Send to Telegram chat.
    Telegram,
    /// Send webhook POST request.
    Webhook,
    /// macOS/Linux desktop notification.
    Desktop,
}

impl Default for NotifyChannel {
    fn default() -> Self {
        NotifyChannel::Log
    }
}

/// Parameters for notify_send tool.
#[derive(Debug, Deserialize)]
pub(crate) struct NotifySendParams {
    /// Notification channel.
    #[serde(default)]
    pub channel: NotifyChannel,
    /// Message content.
    pub message: String,
    /// Optional title/subject.
    #[serde(default)]
    pub title: Option<String>,
    /// Priority level (low, normal, high, urgent).
    #[serde(default = "default_priority")]
    pub priority: String,
    /// Additional channel-specific options.
    #[serde(default)]
    pub options: serde_json::Value,
}

pub(crate) fn default_priority() -> String {
    "normal".to_string()
}

/// Response from notify_send.
#[derive(Debug, Serialize)]
pub(crate) struct NotifySendResponse {
    /// Whether the notification was sent successfully.
    pub success: bool,
    /// Channel used.
    pub channel: String,
    /// Status message.
    pub message: String,
    /// Delivery ID if available.
    pub delivery_id: Option<String>,
}
