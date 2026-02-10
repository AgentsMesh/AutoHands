//! Tests for alert types, channels and manager.

use super::*;
use crate::alert_channels::{EmailChannel, SlackChannel, TelegramChannel};
use crate::alert_manager::AlertManager;
use crate::config::{AlertsConfig, EmailConfig};

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
    assert!(md.contains("\u{26a0}\u{fe0f}"));
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
    assert_eq!(AlertSeverity::Info.emoji(), "\u{2139}\u{fe0f}");
    assert_eq!(AlertSeverity::Warning.emoji(), "\u{26a0}\u{fe0f}");
    assert_eq!(AlertSeverity::Error.emoji(), "\u{274c}");
    assert_eq!(AlertSeverity::Critical.emoji(), "\u{1f6a8}");
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
