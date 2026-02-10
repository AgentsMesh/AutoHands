use super::*;
use tempfile::TempDir;

fn create_test_context(work_dir: PathBuf) -> ToolContext {
    ToolContext::new("test", work_dir)
}

#[test]
fn test_tool_definition() {
    let tool = ListDirectoryTool::new();
    assert_eq!(tool.definition().id, "list_directory");
    assert_eq!(tool.definition().risk_level, RiskLevel::Low);
}

#[test]
fn test_default() {
    let tool = ListDirectoryTool::default();
    assert_eq!(tool.definition().id, "list_directory");
}

#[test]
fn test_default_depth() {
    assert_eq!(default_depth(), 1);
}

#[tokio::test]
async fn test_list_directory_success() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
    std::fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
    std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("file1.txt"));
    assert!(result.content.contains("file2.txt"));
    assert!(result.content.contains("subdir"));
}

#[tokio::test]
async fn test_list_directory_with_depth() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("a/b/c")).unwrap();
    std::fs::write(temp_dir.path().join("a/file.txt"), "").unwrap();
    std::fs::write(temp_dir.path().join("a/b/nested.txt"), "").unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": temp_dir.path().to_str().unwrap(),
        "depth": 3
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("file.txt"));
    assert!(result.content.contains("nested.txt"));
}

#[tokio::test]
async fn test_list_directory_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "/nonexistent/directory"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ResourceNotFound(_) => {}
        e => panic!("Expected ResourceNotFound, got {:?}", e),
    }
}

#[tokio::test]
async fn test_list_directory_not_a_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file.txt");
    std::fs::write(&file_path, "content").unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ExecutionFailed(msg) => assert!(msg.contains("Not a directory")),
        e => panic!("Expected ExecutionFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_list_directory_relative_path() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    std::fs::write(temp_dir.path().join("subdir/file.txt"), "").unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "subdir"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("file.txt"));
}

#[test]
fn test_list_directory_tool_name() {
    let tool = ListDirectoryTool::new();
    assert_eq!(tool.definition().name, "List Directory");
}

#[test]
fn test_list_directory_params_defaults() {
    let json = serde_json::json!({
        "path": "/tmp"
    });
    let params: ListDirectoryParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "/tmp");
    assert_eq!(params.depth, 1);
}

#[test]
fn test_list_directory_params_custom_depth() {
    let json = serde_json::json!({
        "path": "/tmp",
        "depth": 5
    });
    let params: ListDirectoryParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.depth, 5);
}

#[test]
fn test_resolve_path_absolute() {
    let work_dir = PathBuf::from("/work");
    let result = resolve_path("/absolute/path", &work_dir);
    assert_eq!(result, PathBuf::from("/absolute/path"));
}

#[test]
fn test_resolve_path_relative() {
    let work_dir = PathBuf::from("/work");
    let result = resolve_path("relative/path", &work_dir);
    assert_eq!(result, PathBuf::from("/work/relative/path"));
}

#[tokio::test]
async fn test_list_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir(&empty_dir).unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": empty_dir.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.is_empty());
}

#[tokio::test]
async fn test_list_directory_invalid_params() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "invalid": "params"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_directory_depth_zero() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("file.txt"), "").unwrap();

    let tool = ListDirectoryTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": temp_dir.path().to_str().unwrap(),
        "depth": 0
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    // depth 0 means only the root directory itself
    assert!(result.content.is_empty());
}
