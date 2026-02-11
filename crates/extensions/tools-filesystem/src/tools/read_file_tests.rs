use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_context(work_dir: PathBuf) -> ToolContext {
    ToolContext::new("test", work_dir)
}

#[test]
fn test_tool_definition() {
    let tool = ReadFileTool::new();
    assert_eq!(tool.definition().id, "read_file");
    assert_eq!(tool.definition().risk_level, RiskLevel::Low);
}

#[test]
fn test_default() {
    let tool = ReadFileTool::default();
    assert_eq!(tool.definition().id, "read_file");
}

#[tokio::test]
async fn test_read_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("line1"));
    assert!(result.content.contains("line2"));
}

#[tokio::test]
async fn test_read_file_with_offset_and_limit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "offset": 2,
        "limit": 2
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("line2"));
    assert!(result.content.contains("line3"));
    assert!(!result.content.contains("line1"));
    assert!(!result.content.contains("line4"));
}

#[tokio::test]
async fn test_read_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "nonexistent_file.txt"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ResourceNotFound(_) => {}
        e => panic!("Expected ResourceNotFound, got {:?}", e),
    }
}

#[tokio::test]
async fn test_read_file_relative_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("subdir/test.txt");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, "content").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "subdir/test.txt"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("content"));
}

#[tokio::test]
async fn test_read_file_invalid_params() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({});

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_path_traversal_denied() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "../../../etc/passwd"
    });
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Path traversal denied"));
}

#[test]
fn test_read_file_params_defaults() {
    let json = serde_json::json!({
        "path": "test.txt"
    });
    let params: ReadFileParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "test.txt");
    assert!(params.offset.is_none());
    assert!(params.limit.is_none());
}

#[test]
fn test_read_file_params_with_options() {
    let json = serde_json::json!({
        "path": "test.txt",
        "offset": 10,
        "limit": 20
    });
    let params: ReadFileParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.offset, Some(10));
    assert_eq!(params.limit, Some(20));
}

#[test]
fn test_read_file_tool_name() {
    let tool = ReadFileTool::new();
    assert_eq!(tool.definition().name, "Read File");
}

#[tokio::test]
async fn test_read_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");
    std::fs::write(&file_path, "").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.is_empty());
}

#[tokio::test]
async fn test_read_file_offset_beyond_end() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "offset": 100
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.is_empty());
}

#[tokio::test]
async fn test_read_file_limit_zero() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2").unwrap();

    let tool = ReadFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "limit": 0
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.is_empty());
}
