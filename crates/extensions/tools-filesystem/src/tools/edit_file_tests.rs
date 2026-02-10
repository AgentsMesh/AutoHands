use super::*;
use tempfile::TempDir;

fn create_test_context(work_dir: PathBuf) -> ToolContext {
    ToolContext::new("test", work_dir)
}

#[test]
fn test_tool_definition() {
    let tool = EditFileTool::new();
    assert_eq!(tool.definition().id, "edit_file");
    assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
}

#[test]
fn test_default() {
    let tool = EditFileTool::default();
    assert_eq!(tool.definition().id, "edit_file");
}

#[tokio::test]
async fn test_edit_file_single_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, World!").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "World",
        "new_string": "Rust"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("1 replacement"));

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello, Rust!");
}

#[tokio::test]
async fn test_edit_file_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "foo bar foo baz foo").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "foo",
        "new_string": "qux",
        "replace_all": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("3 replacements"));

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "qux bar qux baz qux");
}

#[tokio::test]
async fn test_edit_file_text_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, World!").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "NotFound",
        "new_string": "Replacement"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ExecutionFailed(msg) => assert!(msg.contains("not found")),
        e => panic!("Expected ExecutionFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_edit_file_multiple_matches_without_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "foo bar foo").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "foo",
        "new_string": "baz"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ExecutionFailed(msg) => assert!(msg.contains("2 times")),
        e => panic!("Expected ExecutionFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_edit_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "/nonexistent/file.txt",
        "old_string": "foo",
        "new_string": "bar"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ResourceNotFound(_) => {}
        e => panic!("Expected ResourceNotFound, got {:?}", e),
    }
}

#[tokio::test]
async fn test_edit_file_invalid_params() {
    let temp_dir = TempDir::new().unwrap();
    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "/test.txt"
        // missing old_string and new_string
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_edit_file_tool_name() {
    let tool = EditFileTool::new();
    assert_eq!(tool.definition().name, "Edit File");
}

#[test]
fn test_edit_file_params_parsing() {
    let json = serde_json::json!({
        "path": "test.txt",
        "old_string": "old",
        "new_string": "new"
    });
    let params: EditFileParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "test.txt");
    assert_eq!(params.old_string, "old");
    assert_eq!(params.new_string, "new");
    assert!(!params.replace_all);
}

#[test]
fn test_edit_file_params_with_replace_all() {
    let json = serde_json::json!({
        "path": "test.txt",
        "old_string": "old",
        "new_string": "new",
        "replace_all": true
    });
    let params: EditFileParams = serde_json::from_value(json).unwrap();
    assert!(params.replace_all);
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
async fn test_edit_file_relative_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("subdir/test.txt");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, "Hello, World!").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": "subdir/test.txt",
        "old_string": "World",
        "new_string": "Rust"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello, Rust!");
}

#[tokio::test]
async fn test_edit_file_empty_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, World!").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": ", World",
        "new_string": ""
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello!");
}

#[tokio::test]
async fn test_edit_file_multiline() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let tool = EditFileTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());
    let params = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "line2",
        "new_string": "modified"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "line1\nmodified\nline3");
}
