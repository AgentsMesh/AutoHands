//! Alert types and core trait definitions.

#[cfg(test)]
#[path = "alerts_tests.rs"]
mod tests;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

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
            AlertSeverity::Info => "\u{2139}\u{fe0f}",
            AlertSeverity::Warning => "\u{26a0}\u{fe0f}",
            AlertSeverity::Error => "\u{274c}",
            AlertSeverity::Critical => "\u{1f6a8}",
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
