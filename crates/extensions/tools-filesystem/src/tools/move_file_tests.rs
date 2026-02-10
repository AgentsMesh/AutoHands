use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_move_file() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    tokio::fs::write(&source, "content").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "source.txt",
        "destination": "dest.txt"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));
    assert!(!source.exists());
    assert!(temp.path().join("dest.txt").exists());
}

#[tokio::test]
async fn test_rename_file() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old_name.txt");
    tokio::fs::write(&source, "content").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "old_name.txt",
        "destination": "new_name.txt"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));
    assert!(!source.exists());
    assert!(temp.path().join("new_name.txt").exists());
}

#[tokio::test]
async fn test_move_to_new_directory() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("file.txt");
    tokio::fs::write(&source, "content").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "file.txt",
        "destination": "new_dir/file.txt"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));
    assert!(temp.path().join("new_dir/file.txt").exists());
}

#[tokio::test]
async fn test_move_overwrite() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("source.txt"), "new content").await.unwrap();
    tokio::fs::write(temp.path().join("dest.txt"), "old content").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "source.txt",
        "destination": "dest.txt",
        "overwrite": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));

    let content = tokio::fs::read_to_string(temp.path().join("dest.txt")).await.unwrap();
    assert_eq!(content, "new content");
}

#[tokio::test]
async fn test_move_no_overwrite_error() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("source.txt"), "content").await.unwrap();
    tokio::fs::write(temp.path().join("dest.txt"), "existing").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "source.txt",
        "destination": "dest.txt"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_move_file_tool_default() {
    let tool = MoveFileTool::default();
    assert_eq!(tool.definition().id, "move_file");
}

#[test]
fn test_move_file_tool_definition() {
    let tool = MoveFileTool::new();
    assert_eq!(tool.definition().name, "Move File");
    assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
}

#[test]
fn test_move_file_params_parsing() {
    let json = serde_json::json!({
        "source": "/tmp/src.txt",
        "destination": "/tmp/dest.txt"
    });
    let params: MoveFileParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.source, "/tmp/src.txt");
    assert_eq!(params.destination, "/tmp/dest.txt");
    assert!(!params.overwrite);
}

#[test]
fn test_move_file_params_with_overwrite() {
    let json = serde_json::json!({
        "source": "src",
        "destination": "dest",
        "overwrite": true
    });
    let params: MoveFileParams = serde_json::from_value(json).unwrap();
    assert!(params.overwrite);
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
async fn test_move_source_not_found() {
    let temp = TempDir::new().unwrap();
    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "nonexistent.txt",
        "destination": "dest.txt"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_move_invalid_params() {
    let temp = TempDir::new().unwrap();
    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "invalid": "params"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_move_overwrite_directory() {
    let temp = TempDir::new().unwrap();

    // Create source file
    tokio::fs::write(temp.path().join("source.txt"), "content").await.unwrap();

    // Create destination directory
    let dest_dir = temp.path().join("dest_dir");
    tokio::fs::create_dir(&dest_dir).await.unwrap();
    tokio::fs::write(dest_dir.join("inner.txt"), "inner").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "source.txt",
        "destination": "dest_dir",
        "overwrite": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));
}

#[tokio::test]
async fn test_move_directory() {
    let temp = TempDir::new().unwrap();

    // Create source directory with file
    let source_dir = temp.path().join("source_dir");
    tokio::fs::create_dir(&source_dir).await.unwrap();
    tokio::fs::write(source_dir.join("file.txt"), "content").await.unwrap();

    let tool = MoveFileTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());

    let params = serde_json::json!({
        "source": "source_dir",
        "destination": "dest_dir"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Moved"));
    assert!(!source_dir.exists());
    assert!(temp.path().join("dest_dir").exists());
    assert!(temp.path().join("dest_dir/file.txt").exists());
}
