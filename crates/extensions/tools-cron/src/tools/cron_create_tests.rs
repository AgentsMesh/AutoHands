use super::*;
use std::path::PathBuf;

fn create_test_context() -> ToolContext {
    ToolContext::new("test", PathBuf::from("/tmp"))
}

#[test]
fn test_tool_definition() {
    let tool = CronCreateTool::new();
    assert_eq!(tool.definition().id, "cron_create");
    assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn test_create_cron_job() {
    let tool = CronCreateTool::new();
    let ctx = create_test_context();
    let params = serde_json::json!({
        "name": "daily-backup",
        "schedule": "0 0 0 * * *",
        "command": "backup all files",
        "description": "Daily backup task"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("daily-backup"));
    assert!(result.content.contains("enabled and will run"));
}

#[tokio::test]
async fn test_create_disabled_cron_job() {
    let tool = CronCreateTool::new();
    let ctx = create_test_context();
    let params = serde_json::json!({
        "name": "disabled-task",
        "schedule": "0 */5 * * * *",
        "command": "some command",
        "enabled": false
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("disabled"));
}

#[tokio::test]
async fn test_invalid_cron_expression() {
    let tool = CronCreateTool::new();
    let ctx = create_test_context();
    let params = serde_json::json!({
        "name": "invalid-task",
        "schedule": "invalid cron",
        "command": "some command"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::InvalidParameters(msg) => {
            assert!(msg.contains("Invalid cron expression"));
        }
        e => panic!("Expected InvalidParameters, got {:?}", e),
    }
}

#[tokio::test]
async fn test_missing_required_params() {
    let tool = CronCreateTool::new();
    let ctx = create_test_context();
    let params = serde_json::json!({
        "name": "test-task"
        // Missing schedule and command
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_default_enabled() {
    assert!(default_enabled());
}
