//! Alert channel implementations (Slack, Telegram, Email).

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::config::EmailConfig;
use crate::error::MonitorError;

use super::alerts::{Alert, AlertChannel};

/// Slack webhook channel.
pub struct SlackChannel {
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackChannel {
    /// Create a new Slack channel.
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AlertChannel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, alert: &Alert) -> Result<(), MonitorError> {
        let payload = serde_json::json!({
            "attachments": [{
                "color": alert.severity.color(),
                "title": format!("{} {}", alert.severity.emoji(), alert.title),
                "text": alert.message,
                "footer": alert.source.as_deref().unwrap_or("AutoHands"),
                "ts": alert.timestamp.timestamp(),
                "fields": alert.details.as_ref().map(|d| {
                    if let Some(obj) = d.as_object() {
                        obj.iter().map(|(k, v)| {
                            serde_json::json!({
                                "title": k,
                                "value": v.to_string(),
                                "short": true
                            })
                        }).collect::<Vec<_>>()
                    } else {
                        vec![]
                    }
                }).unwrap_or_default()
            }]
        });

        let response = self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MonitorError::Alert(format!("Slack request failed: {}", e)))?;

        if response.status().is_success() {
            debug!("Slack alert sent successfully");
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(MonitorError::Alert(format!(
                "Slack webhook returned {}: {}",
                status, body
            )))
        }
    }
}

/// Telegram bot channel.
pub struct TelegramChannel {
    bot_token: String,
    chat_id: String,
    client: reqwest::Client,
}

impl TelegramChannel {
    /// Create a new Telegram channel.
    pub fn new(bot_token: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            chat_id: chat_id.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AlertChannel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, alert: &Alert) -> Result<(), MonitorError> {
        let text = alert.format_markdown();

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "Markdown"
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MonitorError::Alert(format!("Telegram request failed: {}", e)))?;

        if response.status().is_success() {
            debug!("Telegram alert sent successfully");
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(MonitorError::Alert(format!(
                "Telegram API returned {}: {}",
                status, body
            )))
        }
    }
}

/// Email channel (SMTP).
///
/// Note: This is a basic implementation. For production use,
/// consider using a dedicated email library like `lettre`.
pub struct EmailChannel {
    config: EmailConfig,
}

impl EmailChannel {
    /// Create a new Email channel.
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
        }
    }

}

#[async_trait]
impl AlertChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, alert: &Alert) -> Result<(), MonitorError> {
        warn!(
            "Email channel is using placeholder implementation. \
             Alert '{}' would be sent to {:?}",
            alert.title, self.config.to
        );

        debug!(
            "Email alert details:\n\
             From: {}\n\
             To: {:?}\n\
             Subject: [{}] {}\n\
             Body: {}",
            self.config.from,
            self.config.to,
            alert.severity,
            alert.title,
            alert.format_text()
        );

        Ok(())
    }
}
