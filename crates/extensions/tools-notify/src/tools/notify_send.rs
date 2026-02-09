//! Send notification tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

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
struct NotifySendParams {
    /// Notification channel.
    #[serde(default)]
    channel: NotifyChannel,
    /// Message content.
    message: String,
    /// Optional title/subject.
    #[serde(default)]
    title: Option<String>,
    /// Priority level (low, normal, high, urgent).
    #[serde(default = "default_priority")]
    priority: String,
    /// Additional channel-specific options.
    #[serde(default)]
    options: serde_json::Value,
}

fn default_priority() -> String {
    "normal".to_string()
}

/// Response from notify_send.
#[derive(Debug, Serialize)]
struct NotifySendResponse {
    /// Whether the notification was sent successfully.
    success: bool,
    /// Channel used.
    channel: String,
    /// Status message.
    message: String,
    /// Delivery ID if available.
    delivery_id: Option<String>,
}

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
    fn send_log(&self, title: Option<&str>, message: &str, priority: &str) -> NotifySendResponse {
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

    /// Send notification via Slack webhook.
    async fn send_slack(
        &self,
        title: Option<&str>,
        message: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let env_webhook = std::env::var("SLACK_WEBHOOK_URL").ok();
        let webhook_url = options
            .get("webhook_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(env_webhook);

        let Some(url) = webhook_url else {
            return NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: "Slack webhook URL not configured. Set SLACK_WEBHOOK_URL or provide webhook_url in options.".to_string(),
                delivery_id: None,
            };
        };

        let text = match title {
            Some(t) => format!("*{}*\n{}", t, message),
            None => message.to_string(),
        };

        let payload = serde_json::json!({
            "text": text
        });

        match reqwest::Client::new()
            .post(url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => NotifySendResponse {
                success: true,
                channel: "slack".to_string(),
                message: "Notification sent to Slack".to_string(),
                delivery_id: None,
            },
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: format!("Slack API error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: format!("Failed to send Slack notification: {}", e),
                delivery_id: None,
            },
        }
    }

    /// Send notification via Telegram.
    async fn send_telegram(
        &self,
        title: Option<&str>,
        message: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
        let env_chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();
        let chat_id = options
            .get("to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(env_chat_id);

        let (Some(token), Some(chat)) = (bot_token, chat_id) else {
            return NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: "Telegram not configured. Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID.".to_string(),
                delivery_id: None,
            };
        };

        let text = match title {
            Some(t) => format!("*{}*\n{}", t, message),
            None => message.to_string(),
        };

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            token
        );

        let payload = serde_json::json!({
            "chat_id": chat,
            "text": text,
            "parse_mode": "Markdown"
        });

        match reqwest::Client::new()
            .post(&url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let message_id = body
                    .get("result")
                    .and_then(|r| r.get("message_id"))
                    .and_then(|m| m.as_i64())
                    .map(|m| m.to_string());

                NotifySendResponse {
                    success: true,
                    channel: "telegram".to_string(),
                    message: "Notification sent to Telegram".to_string(),
                    delivery_id: message_id,
                }
            }
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: format!("Telegram API error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: format!("Failed to send Telegram notification: {}", e),
                delivery_id: None,
            },
        }
    }

    /// Send notification via generic webhook.
    async fn send_webhook(
        &self,
        title: Option<&str>,
        message: &str,
        priority: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let webhook_url = options.get("webhook_url").and_then(|v| v.as_str());

        let Some(url) = webhook_url else {
            return NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: "webhook_url is required in options".to_string(),
                delivery_id: None,
            };
        };

        let payload = serde_json::json!({
            "title": title,
            "message": message,
            "priority": priority,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        match reqwest::Client::new()
            .post(url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => NotifySendResponse {
                success: true,
                channel: "webhook".to_string(),
                message: format!("Notification sent to webhook: {}", url),
                delivery_id: None,
            },
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: format!("Webhook error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: format!("Failed to send webhook notification: {}", e),
                delivery_id: None,
            },
        }
    }

    /// Send desktop notification.
    fn send_desktop(&self, title: Option<&str>, message: &str) -> NotifySendResponse {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_context() -> ToolContext {
        ToolContext::new("test", PathBuf::from("/tmp"))
    }

    #[test]
    fn test_tool_definition() {
        let tool = NotifySendTool::new();
        assert_eq!(tool.definition().id, "notify_send");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_send_log_notification() {
        let tool = NotifySendTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "channel": "log",
            "message": "Test notification",
            "title": "Test Title",
            "priority": "high"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("log"));
    }

    #[tokio::test]
    async fn test_send_default_channel() {
        let tool = NotifySendTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "message": "Test notification"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        // Default channel is log
        assert!(result.content.contains("log"));
    }

    #[tokio::test]
    async fn test_missing_message() {
        let tool = NotifySendTool::new();
        let ctx = create_test_context();
        let params = serde_json::json!({
            "channel": "log"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_default_priority() {
        assert_eq!(default_priority(), "normal");
    }

    #[test]
    fn test_notify_channel_default() {
        let channel = NotifyChannel::default();
        matches!(channel, NotifyChannel::Log);
    }
}
