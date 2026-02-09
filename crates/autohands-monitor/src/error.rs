//! Monitor errors.

use thiserror::Error;

/// Monitor error types.
#[derive(Debug, Error)]
pub enum MonitorError {
    /// Failed to collect metrics.
    #[error("Failed to collect metrics: {0}")]
    MetricsCollection(String),

    /// Alert delivery failed.
    #[error("Alert delivery failed: {0}")]
    AlertDelivery(String),

    /// Alert channel error.
    #[error("Alert error: {0}")]
    Alert(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Channel not configured.
    #[error("Alert channel not configured: {0}")]
    ChannelNotConfigured(String),

    /// Generic error.
    #[error("{0}")]
    Custom(String),
}
