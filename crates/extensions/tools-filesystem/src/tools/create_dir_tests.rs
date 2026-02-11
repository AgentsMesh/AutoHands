use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_create_directory() {
    let temp = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "new_dir"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Created directory"));
    assert!(temp.path().join("new_dir").exists());
}

#[tokio::test]
async fn test_create_nested_directory() {
    let temp = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "a/b/c",
        "parents": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Created directory"));
    assert!(temp.path().join("a/b/c").exists());
}

#[tokio::test]
async fn test_create_existing_directory() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("existing")).unwrap();

    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "existing"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("already exists"));
}

#[test]
fn test_create_directory_tool_default() {
    let tool = CreateDirectoryTool::default();
    assert_eq!(tool.definition().id, "create_directory");
}

#[test]
fn test_create_directory_tool_definition() {
    let tool = CreateDirectoryTool::new();
    assert_eq!(tool.definition().name, "Create Directory");
    assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
}

#[test]
fn test_default_true() {
    assert!(default_true());
}

#[test]
fn test_create_dir_params_parsing() {
    let json = serde_json::json!({
        "path": "new_dir"
    });
    let params: CreateDirParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "new_dir");
    assert!(params.parents); // default is true
}

#[test]
fn test_create_dir_params_no_parents() {
    let json = serde_json::json!({
        "path": "new_dir",
        "parents": false
    });
    let params: CreateDirParams = serde_json::from_value(json).unwrap();
    assert!(!params.parents);
}

#[tokio::test]
async fn test_create_dir_path_traversal_denied() {
    let temp_dir = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "../../../tmp/evil_dir"
    });
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Path traversal denied"));
}

#[tokio::test]
async fn test_create_directory_no_parents() {
    let temp = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "single_dir",
        "parents": false
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Created directory"));
    assert!(temp.path().join("single_dir").exists());
}

#[tokio::test]
async fn test_create_nested_without_parents_fails() {
    let temp = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "a/b/c",
        "parents": false
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_directory_invalid_params() {
    let temp = TempDir::new().unwrap();
    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "invalid": "params"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_directory_absolute_path() {
    let temp = TempDir::new().unwrap();
    let abs_path = temp.path().join("abs_dir");

    let tool = CreateDirectoryTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("/"));

    let params = serde_json::json!({
        "path": abs_path.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Created directory"));
    assert!(abs_path.exists());
}
