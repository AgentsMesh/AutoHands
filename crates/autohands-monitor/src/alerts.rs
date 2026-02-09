//! Alert notifications.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::config::{AlertsConfig, EmailConfig};
use crate::error::MonitorError;

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Critical.
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Error => write!(f, "ERROR"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl AlertSeverity {
    /// Get emoji for severity.
    pub fn emoji(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Error => "❌",
            AlertSeverity::Critical => "🚨",
        }
    }

    /// Get color for Slack/Discord.
    pub fn color(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "#36a64f",    // green
            AlertSeverity::Warning => "#f0ad4e", // yellow
            AlertSeverity::Error => "#d9534f",   // red
            AlertSeverity::Critical => "#800000", // dark red
        }
    }
}

/// An alert message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert title.
    pub title: String,
    /// Alert message.
    pub message: String,
    /// Severity level.
    pub severity: AlertSeverity,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Source component.
    pub source: Option<String>,
    /// Additional details.
    pub details: Option<serde_json::Value>,
}

impl Alert {
    /// Create a new alert.
    pub fn new(title: impl Into<String>, message: impl Into<String>, severity: AlertSeverity) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            severity,
            timestamp: Utc::now(),
            source: None,
            details: None,
        }
    }

    /// Set source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Format for text output.
    pub fn format_text(&self) -> String {
        let mut text = format!(
            "[{}] {} - {}\n{}",
            self.severity,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.title,
            self.message
        );

        if let Some(ref source) = self.source {
            text.push_str(&format!("\nSource: {}", source));
        }

        text
    }

    /// Format for Markdown output.
    pub fn format_markdown(&self) -> String {
        let mut text = format!(
            "{} **{}** - {}\n\n{}",
            self.severity.emoji(),
            self.title,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.message
        );

        if let Some(ref source) = self.source {
            text.push_str(&format!("\n\n_Source: {}_", source));
        }

        text
    }
}

/// Alert channel trait.
#[async_trait]
pub trait AlertChannel: Send + Sync {
    /// Channel name.
    fn name(&self) -> &str;

    /// Send an alert.
    async fn send(&self, alert: &Alert) -> Result<(), MonitorError>;
}

/// Log channel (writes to tracing).
pub struct LogChannel;

#[async_trait]
impl AlertChannel for LogChannel {
    fn name(&self) -> &str {
        "log"
    }

    async fn send(&self, alert: &Alert) -> Result<(), MonitorError> {
        match alert.severity {
            AlertSeverity::Info => info!("[ALERT] {}: {}", alert.title, alert.message),
            AlertSeverity::Warning => warn!("[ALERT] {}: {}", alert.title, alert.message),
            AlertSeverity::Error | AlertSeverity::Critical => {
                error!("[ALERT] {}: {}", alert.title, alert.message)
            }
        }
        Ok(())
    }
}

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
    client: reqwest::Client,
}

impl EmailChannel {
    /// Create a new Email channel.
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Format email body as HTML.
    fn format_html(&self, alert: &Alert) -> String {
        let severity_color = alert.severity.color();
        let emoji = alert.severity.emoji();

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; }}
        .alert {{ padding: 20px; border-left: 4px solid {}; background: #f9f9f9; }}
        .title {{ font-size: 18px; font-weight: bold; margin-bottom: 10px; }}
        .message {{ color: #333; }}
        .meta {{ color: #666; font-size: 12px; margin-top: 10px; }}
    </style>
</head>
<body>
    <div class="alert">
        <div class="title">{} {}</div>
        <div class="message">{}</div>
        <div class="meta">
            Time: {}<br>
            Source: {}
        </div>
    </div>
</body>
</html>"#,
            severity_color,
            emoji,
            alert.title,
            alert.message,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            alert.source.as_deref().unwrap_or("AutoHands")
        )
    }
}

#[async_trait]
impl AlertChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, alert: &Alert) -> Result<(), MonitorError> {
        // Note: This is a placeholder implementation.
        // For a real implementation, you would use an SMTP library like `lettre`.
        //
        // Example with lettre (not implemented here to avoid dependency):
        // ```
        // use lettre::{Message, SmtpTransport, Transport};
        //
        // let email = Message::builder()
        //     .from(self.config.from.parse().unwrap())
        //     .to(self.config.to[0].parse().unwrap())
        //     .subject(format!("[{}] {}", alert.severity, alert.title))
        //     .body(alert.format_text())
        //     .unwrap();
        //
        // let mailer = SmtpTransport::relay(&self.config.smtp_server)
        //     .unwrap()
        //     .build();
        //
        // mailer.send(&email).unwrap();
        // ```

        warn!(
            "Email channel is using placeholder implementation. \
             Alert '{}' would be sent to {:?}",
            alert.title, self.config.to
        );

        // Log the alert details for debugging
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

/// Alert manager.
pub struct AlertManager {
    channels: Vec<Box<dyn AlertChannel>>,
}

impl AlertManager {
    /// Create a new alert manager.
    pub fn new() -> Self {
        Self {
            channels: vec![Box::new(LogChannel)],
        }
    }

    /// Create from config.
    pub fn from_config(config: &AlertsConfig) -> Self {
        let mut manager = Self::new();

        // Add Slack channel if configured
        if let Some(ref webhook_url) = config.slack_webhook {
            if !webhook_url.is_empty() {
                info!("Adding Slack alert channel");
                manager.add_channel(Box::new(SlackChannel::new(webhook_url)));
            }
        }

        // Add Telegram channel if configured
        if let (Some(bot_token), Some(chat_id)) =
            (&config.telegram_bot_token, &config.telegram_chat_id)
        {
            if !bot_token.is_empty() && !chat_id.is_empty() {
                info!("Adding Telegram alert channel");
                manager.add_channel(Box::new(TelegramChannel::new(bot_token, chat_id)));
            }
        }

        // Add Email channel if configured
        if let Some(ref email_config) = config.email {
            info!("Adding Email alert channel");
            manager.add_channel(Box::new(EmailChannel::new(email_config.clone())));
        }

        manager
    }

    /// Add a channel.
    pub fn add_channel(&mut self, channel: Box<dyn AlertChannel>) {
        self.channels.push(channel);
    }

    /// Get list of channel names.
    pub fn channel_names(&self) -> Vec<&str> {
        self.channels.iter().map(|c| c.name()).collect()
    }

    /// Send an alert to all channels.
    pub async fn send(&self, alert: &Alert) -> Vec<MonitorError> {
        let mut errors = Vec::new();

        for channel in &self.channels {
            if let Err(e) = channel.send(alert).await {
                error!("Failed to send alert via {}: {}", channel.name(), e);
                errors.push(e);
            }
        }

        errors
    }

    /// Send an info alert.
    pub async fn info(&self, title: impl Into<String>, message: impl Into<String>) {
        let alert = Alert::new(title, message, AlertSeverity::Info);
        self.send(&alert).await;
    }

    /// Send a warning alert.
    pub async fn warning(&self, title: impl Into<String>, message: impl Into<String>) {
        let alert = Alert::new(title, message, AlertSeverity::Warning);
        self.send(&alert).await;
    }

    /// Send an error alert.
    pub async fn error(&self, title: impl Into<String>, message: impl Into<String>) {
        let alert = Alert::new(title, message, AlertSeverity::Error);
        self.send(&alert).await;
    }

    /// Send a critical alert.
    pub async fn critical(&self, title: impl Into<String>, message: impl Into<String>) {
        let alert = Alert::new(title, message, AlertSeverity::Critical);
        self.send(&alert).await;
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_new() {
        let alert = Alert::new("Test Alert", "This is a test", AlertSeverity::Warning);
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_alert_format_text() {
        let alert = Alert::new("Test", "Message", AlertSeverity::Error)
            .with_source("component");

        let text = alert.format_text();
        assert!(text.contains("[ERROR]"));
        assert!(text.contains("Test"));
        assert!(text.contains("Message"));
        assert!(text.contains("Source: component"));
    }

    #[test]
    fn test_alert_format_markdown() {
        let alert = Alert::new("Test", "Message", AlertSeverity::Warning)
            .with_source("component");

        let md = alert.format_markdown();
        assert!(md.contains("⚠️"));
        assert!(md.contains("**Test**"));
        assert!(md.contains("Message"));
        assert!(md.contains("_Source: component_"));
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "INFO");
        assert_eq!(AlertSeverity::Warning.to_string(), "WARNING");
        assert_eq!(AlertSeverity::Error.to_string(), "ERROR");
        assert_eq!(AlertSeverity::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_severity_emoji() {
        assert_eq!(AlertSeverity::Info.emoji(), "ℹ️");
        assert_eq!(AlertSeverity::Warning.emoji(), "⚠️");
        assert_eq!(AlertSeverity::Error.emoji(), "❌");
        assert_eq!(AlertSeverity::Critical.emoji(), "🚨");
    }

    #[test]
    fn test_severity_color() {
        assert_eq!(AlertSeverity::Info.color(), "#36a64f");
        assert_eq!(AlertSeverity::Warning.color(), "#f0ad4e");
        assert_eq!(AlertSeverity::Error.color(), "#d9534f");
        assert_eq!(AlertSeverity::Critical.color(), "#800000");
    }

    #[tokio::test]
    async fn test_alert_manager_send() {
        let manager = AlertManager::new();
        let alert = Alert::new("Test", "Test message", AlertSeverity::Info);

        let errors = manager.send(&alert).await;
        assert!(errors.is_empty());
    }

    #[test]
    fn test_alert_manager_from_config() {
        let config = AlertsConfig {
            slack_webhook: Some("https://hooks.slack.com/test".to_string()),
            telegram_bot_token: Some("123456:ABC".to_string()),
            telegram_chat_id: Some("123456".to_string()),
            email: None,
        };

        let manager = AlertManager::from_config(&config);
        let names = manager.channel_names();

        assert!(names.contains(&"log"));
        assert!(names.contains(&"slack"));
        assert!(names.contains(&"telegram"));
    }

    #[test]
    fn test_alert_manager_channel_names() {
        let manager = AlertManager::new();
        let names = manager.channel_names();

        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "log");
    }

    #[test]
    fn test_slack_channel_new() {
        let channel = SlackChannel::new("https://hooks.slack.com/test");
        assert_eq!(channel.name(), "slack");
    }

    #[test]
    fn test_telegram_channel_new() {
        let channel = TelegramChannel::new("token", "chat_id");
        assert_eq!(channel.name(), "telegram");
    }

    #[test]
    fn test_email_channel_new() {
        let config = EmailConfig {
            smtp_server: "smtp.example.com".to_string(),
            smtp_port: 587,
            from: "alerts@example.com".to_string(),
            to: vec!["admin@example.com".to_string()],
            username: None,
            password: None,
        };

        let channel = EmailChannel::new(config);
        assert_eq!(channel.name(), "email");
    }
}
