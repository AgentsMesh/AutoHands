//! Send notification tool implementation.

#[cfg(test)]
#[path = "notify_send_tests.rs"]
mod tests;

use async_trait::async_trait;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use super::notify_types::{NotifyChannel, NotifySendParams, NotifySendResponse};

/// Send notification tool implementation.
pub struct NotifySendTool {
    definition: ToolDefinition,
}

impl NotifySendTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "enum": ["log", "email", "slack", "telegram", "webhook", "desktop"],
                    "description": "Notification channel to use (default: log)"
                },
                "message": {
                    "type": "string",
                    "description": "The notification message content"
                },
                "title": {
                    "type": "string",
                    "description": "Optional title or subject for the notification"
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "normal", "high", "urgent"],
                    "description": "Priority level (default: normal)"
                },
                "options": {
                    "type": "object",
                    "description": "Channel-specific options (e.g., email recipients, Slack channel)",
                    "properties": {
                        "to": {
                            "type": "string",
                            "description": "Recipient (email address, Slack channel, Telegram chat ID)"
                        },
                        "webhook_url": {
                            "type": "string",
                            "description": "Webhook URL for webhook channel"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["text", "markdown", "html"],
                            "description": "Message format"
                        }
                    }
                }
            },
            "required": ["message"]
        });

        Self {
            definition: ToolDefinition::new(
                "notify_send",
                "Send Notification",
                "Send a notification message through a configured channel",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
        }
    }

    /// Send notification via log.
    pub(crate) fn send_log(
        &self,
        title: Option<&str>,
        message: &str,
        priority: &str,
    ) -> NotifySendResponse {
        let log_message = match title {
            Some(t) => format!("[{}] {}: {}", priority.to_uppercase(), t, message),
            None => format!("[{}] {}", priority.to_uppercase(), message),
        };

        match priority {
            "urgent" | "high" => tracing::warn!("{}", log_message),
            "low" => tracing::debug!("{}", log_message),
            _ => tracing::info!("{}", log_message),
        }

        NotifySendResponse {
            success: true,
            channel: "log".to_string(),
            message: "Notification logged".to_string(),
            delivery_id: None,
        }
    }

    /// Send desktop notification.
    pub(crate) fn send_desktop(
        &self,
        title: Option<&str>,
        message: &str,
    ) -> NotifySendResponse {
        #[cfg(target_os = "macos")]
        {
            let title_str = title.unwrap_or("AutoHands");
            let script = format!(
                r#"display notification "{}" with title "{}""#,
                message.replace('"', r#"\""#),
                title_str.replace('"', r#"\""#)
            );

            match std::process::Command::new("osascript")
                .args(["-e", &script])
                .output()
            {
                Ok(output) if output.status.success() => NotifySendResponse {
                    success: true,
                    channel: "desktop".to_string(),
                    message: "Desktop notification sent".to_string(),
                    delivery_id: None,
                },
                Ok(output) => NotifySendResponse {
                    success: false,
                    channel: "desktop".to_string(),
                    message: format!(
                        "Failed to send desktop notification: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    delivery_id: None,
                },
                Err(e) => NotifySendResponse {
                    success: false,
                    channel: "desktop".to_string(),
                    message: format!("Failed to send desktop notification: {}", e),
                    delivery_id: None,
                },
            }
        }

        #[cfg(target_os = "linux")]
        {
            let title_str = title.unwrap_or("AutoHands");
            match std::process::Command::new("notify-send")
                .args([title_str, message])
                .output()
            {
                Ok(output) if output.status.success() => NotifySendResponse {
                    success: true,
                    channel: "desktop".to_string(),
                    message: "Desktop notification sent".to_string(),
                    delivery_id: None,
                },
                Ok(output) => NotifySendResponse {
                    success: false,
                    channel: "desktop".to_string(),
                    message: format!(
                        "Failed to send desktop notification: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    delivery_id: None,
                },
                Err(e) => NotifySendResponse {
                    success: false,
                    channel: "desktop".to_string(),
                    message: format!("Failed to send desktop notification: {}", e),
                    delivery_id: None,
                },
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            NotifySendResponse {
                success: false,
                channel: "desktop".to_string(),
                message: "Desktop notifications not supported on this platform".to_string(),
                delivery_id: None,
            }
        }
    }
}

impl Default for NotifySendTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NotifySendTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: NotifySendParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let response = match params.channel {
            NotifyChannel::Log => {
                self.send_log(params.title.as_deref(), &params.message, &params.priority)
            }
            NotifyChannel::Slack => {
                self.send_slack(params.title.as_deref(), &params.message, &params.options)
                    .await
            }
            NotifyChannel::Telegram => {
                self.send_telegram(params.title.as_deref(), &params.message, &params.options)
                    .await
            }
            NotifyChannel::Webhook => {
                self.send_webhook(
                    params.title.as_deref(),
                    &params.message,
                    &params.priority,
                    &params.options,
                )
                .await
            }
            NotifyChannel::Desktop => {
                self.send_desktop(params.title.as_deref(), &params.message)
            }
            NotifyChannel::Email => NotifySendResponse {
                success: false,
                channel: "email".to_string(),
                message: "Email notifications not yet implemented".to_string(),
                delivery_id: None,
            },
        };

        if response.success {
            Ok(ToolResult::success(
                serde_json::to_string_pretty(&response).unwrap(),
            ))
        } else {
            Ok(ToolResult::error(&response.message))
        }
    }
}
