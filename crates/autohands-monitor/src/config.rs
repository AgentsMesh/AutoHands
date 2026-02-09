//! Monitor configuration.

use serde::{Deserialize, Serialize};

/// Monitor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Whether monitoring is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Health endpoint path.
    #[serde(default = "default_health_endpoint")]
    pub health_endpoint: String,

    /// Metrics endpoint path.
    #[serde(default = "default_metrics_endpoint")]
    pub metrics_endpoint: String,

    /// Metrics collection interval in seconds.
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval_secs: u64,

    /// Alert channels.
    #[serde(default)]
    pub alerts: AlertsConfig,
}

/// Alert channels configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertsConfig {
    /// Slack webhook URL.
    pub slack_webhook: Option<String>,

    /// Telegram bot token.
    pub telegram_bot_token: Option<String>,

    /// Telegram chat ID.
    pub telegram_chat_id: Option<String>,

    /// Email settings.
    pub email: Option<EmailConfig>,
}

/// Email alert configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP server.
    pub smtp_server: String,
    /// SMTP port.
    pub smtp_port: u16,
    /// From address.
    pub from: String,
    /// To addresses.
    pub to: Vec<String>,
    /// SMTP username.
    pub username: Option<String>,
    /// SMTP password.
    pub password: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_health_endpoint() -> String {
    "/health".to_string()
}

fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

fn default_metrics_interval() -> u64 {
    60
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            health_endpoint: default_health_endpoint(),
            metrics_endpoint: default_metrics_endpoint(),
            metrics_interval_secs: default_metrics_interval(),
            alerts: AlertsConfig::default(),
        }
    }
}
