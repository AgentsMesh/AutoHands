use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_delete_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    tokio::fs::write(&file_path, "content").await.unwrap();

    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "test.txt"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Deleted file"));
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_empty_directory() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("empty_dir");
    tokio::fs::create_dir(&dir_path).await.unwrap();

    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "empty_dir"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Deleted directory"));
    assert!(!dir_path.exists());
}

#[tokio::test]
async fn test_delete_directory_recursive() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("dir");
    tokio::fs::create_dir(&dir_path).await.unwrap();
    tokio::fs::write(dir_path.join("file.txt"), "content").await.unwrap();

    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "dir",
        "recursive": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Deleted directory"));
    assert!(!dir_path.exists());
}

#[tokio::test]
async fn test_delete_nonexistent() {
    let temp = TempDir::new().unwrap();
    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "nonexistent.txt"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_delete_file_tool_default() {
    let tool = DeleteFileTool::default();
    assert_eq!(tool.definition().id, "delete_file");
}

#[test]
fn test_delete_file_tool_definition() {
    let tool = DeleteFileTool::new();
    assert_eq!(tool.definition().name, "Delete File");
    assert_eq!(tool.definition().risk_level, RiskLevel::High);
}

#[test]
fn test_delete_file_params_parsing() {
    let json = serde_json::json!({
        "path": "/tmp/test.txt"
    });
    let params: DeleteFileParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "/tmp/test.txt");
    assert!(!params.recursive);
}

#[test]
fn test_delete_file_params_with_recursive() {
    let json = serde_json::json!({
        "path": "/tmp/dir",
        "recursive": true
    });
    let params: DeleteFileParams = serde_json::from_value(json).unwrap();
    assert!(params.recursive);
}

#[test]
fn test_resolve_path_absolute() {
    let work_dir = PathBuf::from("/home/user");
    let resolved = resolve_path("/absolute/path", &work_dir);
    assert_eq!(resolved, PathBuf::from("/absolute/path"));
}

#[test]
fn test_resolve_path_relative() {
    let work_dir = PathBuf::from("/home/user");
    let resolved = resolve_path("relative/path", &work_dir);
    assert_eq!(resolved, PathBuf::from("/home/user/relative/path"));
}

#[tokio::test]
async fn test_delete_non_empty_dir_without_recursive() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("non_empty_dir");
    tokio::fs::create_dir(&dir_path).await.unwrap();
    tokio::fs::write(dir_path.join("file.txt"), "content").await.unwrap();

    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "path": "non_empty_dir",
        "recursive": false
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_file_with_absolute_path() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("absolute_test.txt");
    tokio::fs::write(&file_path, "content").await.unwrap();

    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("/"));

    let params = serde_json::json!({
        "path": file_path.to_str().unwrap()
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Deleted file"));
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_invalid_params() {
    let temp = TempDir::new().unwrap();
    let tool = DeleteFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "invalid": "params"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}
