use super::*;
use tempfile::TempDir;

#[test]
fn test_glob_tool_definition() {
    let tool = GlobTool::new();
    assert_eq!(tool.definition().id, "glob");
    assert_eq!(tool.definition().name, "Glob Search");
}

#[test]
fn test_glob_tool_default() {
    let tool = GlobTool::default();
    assert_eq!(tool.definition().id, "glob");
}

#[tokio::test]
async fn test_glob_no_matches() {
    let temp = TempDir::new().unwrap();
    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "nonexistent_pattern_xyz/**" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("No files found"));
}

#[tokio::test]
async fn test_glob_with_matches() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.rs"), "content").await.unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "content").await.unwrap();

    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "*.rs" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Found 1 files"));
    assert!(result.content.contains("test.rs"));
}

#[tokio::test]
async fn test_glob_with_path() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("sub");
    tokio::fs::create_dir(&subdir).await.unwrap();
    tokio::fs::write(subdir.join("file.rs"), "content").await.unwrap();

    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({
        "pattern": "*.rs",
        "path": subdir.to_string_lossy()
    });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Found 1 files"));
}

#[tokio::test]
async fn test_glob_invalid_pattern() {
    let temp = TempDir::new().unwrap();
    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "[invalid" });
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_glob_params_parsing() {
    let json = serde_json::json!({
        "pattern": "*.rs"
    });
    let params: GlobParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.pattern, "*.rs");
    assert!(params.path.is_none());
}

#[test]
fn test_glob_params_with_path() {
    let json = serde_json::json!({
        "pattern": "*.rs",
        "path": "/tmp/src"
    });
    let params: GlobParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, Some("/tmp/src".to_string()));
}

#[test]
fn test_glob_tool_risk_level() {
    let tool = GlobTool::new();
    assert_eq!(tool.definition().risk_level, RiskLevel::Low);
}

#[tokio::test]
async fn test_glob_recursive_pattern() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("sub/nested");
    tokio::fs::create_dir_all(&subdir).await.unwrap();
    tokio::fs::write(subdir.join("deep.rs"), "content").await.unwrap();
    tokio::fs::write(temp.path().join("top.rs"), "content").await.unwrap();

    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "**/*.rs" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Found"));
    assert!(result.content.contains("top.rs") || result.content.contains("deep.rs"));
}

#[tokio::test]
async fn test_glob_multiple_matches() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("a.rs"), "content").await.unwrap();
    tokio::fs::write(temp.path().join("b.rs"), "content").await.unwrap();
    tokio::fs::write(temp.path().join("c.rs"), "content").await.unwrap();

    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "*.rs" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("Found 3 files"));
}

#[tokio::test]
async fn test_glob_invalid_params() {
    let temp = TempDir::new().unwrap();
    let tool = GlobTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({});
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}
