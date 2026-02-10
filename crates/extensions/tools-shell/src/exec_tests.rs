use super::*;
use tempfile::TempDir;

fn create_test_context(work_dir: std::path::PathBuf) -> ToolContext {
    ToolContext::new("test", work_dir)
}

#[test]
fn test_tool_definition() {
    let tool = ExecTool::new();
    assert_eq!(tool.definition().id, "exec");
    assert_eq!(tool.definition().risk_level, RiskLevel::High);
}

#[test]
fn test_default() {
    let tool = ExecTool::default();
    assert_eq!(tool.definition().id, "exec");
}

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 120_000);
}

#[tokio::test]
async fn test_exec_echo() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "echo hello"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("hello"));
}

#[tokio::test]
async fn test_exec_with_cwd() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("test.txt"), "content").unwrap();

    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "ls",
        "cwd": subdir.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("test.txt"));
}

#[tokio::test]
async fn test_exec_failure() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "exit 1"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("exit code 1"));
}

#[tokio::test]
async fn test_exec_stderr() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "echo error >&2"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("error"));
}

#[tokio::test]
async fn test_exec_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "sleep 10",
        "timeout": 100  // 100ms timeout
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::Timeout(_) => {}
        e => panic!("Expected Timeout, got {:?}", e),
    }
}

#[tokio::test]
async fn test_exec_invalid_params() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({});

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_exec_multiline_output() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "command": "echo line1; echo line2; echo line3"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("line1"));
    assert!(result.content.contains("line2"));
    assert!(result.content.contains("line3"));
}
