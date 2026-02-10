//! Tests for notify_send tool.

use super::*;
use crate::tools::notify_types::{default_priority, NotifyChannel};
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
