use super::*;

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 30_000);
}

#[test]
fn test_session_params_parsing() {
    let json = serde_json::json!({
        "action": "create"
    });
    let params: SessionParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.action, "create");
    assert!(params.session_id.is_none());
    assert!(params.command.is_none());
    assert_eq!(params.timeout, 30_000);
}

#[test]
fn test_session_params_with_all_fields() {
    let json = serde_json::json!({
        "action": "execute",
        "session_id": "session_1",
        "command": "echo hello",
        "timeout": 60000
    });
    let params: SessionParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.action, "execute");
    assert_eq!(params.session_id, Some("session_1".to_string()));
    assert_eq!(params.command, Some("echo hello".to_string()));
    assert_eq!(params.timeout, 60000);
}

#[test]
fn test_tool_definition() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    assert_eq!(tool.definition().id, "shell_session");
    assert_eq!(tool.definition().risk_level, RiskLevel::High);
}

#[tokio::test]
async fn test_create_session() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "create"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("Session created"));
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "list"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("No active sessions"));
}

#[tokio::test]
async fn test_execute_missing_session_id() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "execute",
        "command": "echo hello"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_missing_command() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "execute",
        "session_id": "session_1"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_kill_missing_session_id() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({
        "action": "kill"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unknown_action() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
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
async fn test_invalid_params() {
    let manager = Arc::new(SessionManager::new());
    let tool = SessionTool::new(manager);
    let ctx = ToolContext::new("test", std::env::current_dir().unwrap());

    let params = serde_json::json!({});

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}
