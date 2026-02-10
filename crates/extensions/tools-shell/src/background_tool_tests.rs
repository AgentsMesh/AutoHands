use super::*;

#[test]
fn test_background_params_parsing() {
    let json = serde_json::json!({
        "action": "list"
    });
    let params: BackgroundParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.action, "list");
    assert!(params.command.is_none());
    assert!(params.process_id.is_none());
    assert!(params.cwd.is_none());
}

#[test]
fn test_background_params_spawn() {
    let json = serde_json::json!({
        "action": "spawn",
        "command": "sleep 10",
        "cwd": "/tmp"
    });
    let params: BackgroundParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.action, "spawn");
    assert_eq!(params.command, Some("sleep 10".to_string()));
    assert_eq!(params.cwd, Some("/tmp".to_string()));
}

#[test]
fn test_background_params_with_process_id() {
    let json = serde_json::json!({
        "action": "status",
        "process_id": "proc_123"
    });
    let params: BackgroundParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.action, "status");
    assert_eq!(params.process_id, Some("proc_123".to_string()));
}

#[test]
fn test_tool_definition() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    assert_eq!(tool.definition().id, "background");
    assert_eq!(tool.definition().risk_level, RiskLevel::High);
}

#[test]
fn test_format_status_running() {
    let status = ProcessStatus::Running;
    assert_eq!(BackgroundTool::format_status(&status), "Running");
}

#[test]
fn test_format_status_completed() {
    let status = ProcessStatus::Completed(0);
    assert_eq!(BackgroundTool::format_status(&status), "Completed (exit code: 0)");

    let status = ProcessStatus::Completed(1);
    assert_eq!(BackgroundTool::format_status(&status), "Completed (exit code: 1)");
}

#[test]
fn test_format_status_failed() {
    let status = ProcessStatus::Failed("error message".to_string());
    assert_eq!(BackgroundTool::format_status(&status), "Failed: error message");
}

#[tokio::test]
async fn test_list_empty() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "list"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("No background processes"));
}

#[tokio::test]
async fn test_spawn_missing_command() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "spawn"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_status_missing_process_id() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "status"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_status_not_found() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "status",
        "process_id": "nonexistent"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ResourceNotFound(_) => {}
        e => panic!("Expected ResourceNotFound, got {:?}", e),
    }
}

#[tokio::test]
async fn test_kill_missing_process_id() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "kill"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wait_missing_process_id() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "wait"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unknown_action() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "invalid"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::InvalidParameters(msg) => assert!(msg.contains("Unknown action")),
        e => panic!("Expected InvalidParameters, got {:?}", e),
    }
}

#[tokio::test]
async fn test_spawn_and_list() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    // Spawn a process
    let spawn_params = serde_json::json!({
        "action": "spawn",
        "command": "sleep 60"
    });

    let result = tool.execute(spawn_params, ctx.clone()).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("Background process started"));

    // List processes
    let list_params = serde_json::json!({
        "action": "list"
    });

    let result = tool.execute(list_params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("sleep 60"));
}

#[tokio::test]
async fn test_invalid_params() {
    let manager = Arc::new(BackgroundManager::new());
    let tool = BackgroundTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({});

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}
