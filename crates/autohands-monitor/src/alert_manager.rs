//! Alert manager for dispatching alerts to channels.

use tracing::{error, info};

use crate::config::AlertsConfig;
use crate::error::MonitorError;

use super::alert_channels::{EmailChannel, SlackChannel, TelegramChannel};
use super::alerts::{Alert, AlertChannel, AlertSeverity, LogChannel};

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
