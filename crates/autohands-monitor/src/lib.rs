//! # AutoHands Monitor
//!
//! System monitoring for 24/7 autonomous agent framework.
//!
//! ## Features
//!
//! - Health check endpoint (/health)
//! - Prometheus format metrics (/metrics)
//! - Alert notifications (email/Slack/Telegram)

pub mod config;
pub mod error;
pub mod health;
pub mod metrics;
pub mod alerts;
pub mod alert_channels;
pub mod alert_manager;

pub use config::MonitorConfig;
pub use error::MonitorError;
pub use health::HealthEndpoint;
pub use metrics::MetricsEndpoint;
pub use alerts::{
    Alert, AlertChannel, AlertSeverity, LogChannel,
};
pub use alert_channels::{EmailChannel, SlackChannel, TelegramChannel};
pub use alert_manager::AlertManager;
