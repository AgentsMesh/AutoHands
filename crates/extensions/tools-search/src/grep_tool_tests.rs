use super::*;
use tempfile::TempDir;

#[test]
fn test_grep_tool_definition() {
    let tool = GrepTool::new();
    assert_eq!(tool.definition().id, "grep");
    assert_eq!(tool.definition().name, "Content Search");
}

#[test]
fn test_grep_tool_default() {
    let tool = GrepTool::default();
    assert_eq!(tool.definition().id, "grep");
}

#[test]
fn test_search_file() {
    let content = "line 1\nfoo bar\nline 3";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 0);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].contains("foo bar"));
}

#[test]
fn test_search_file_with_context() {
    let content = "line 1\nline 2\nfoo bar\nline 4\nline 5";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 1);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].contains("line 2"));
    assert!(matches[0].contains("foo bar"));
    assert!(matches[0].contains("line 4"));
}

#[test]
fn test_search_file_no_match() {
    let content = "line 1\nline 2\nline 3";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 0);
    assert!(matches.is_empty());
}

#[test]
fn test_matches_glob() {
    assert!(matches_glob(std::path::Path::new("test.rs"), "*.rs"));
    assert!(!matches_glob(std::path::Path::new("test.rs"), "*.ts"));
}

#[test]
fn test_matches_glob_invalid() {
    assert!(!matches_glob(std::path::Path::new("test.rs"), "[invalid"));
}

#[tokio::test]
async fn test_grep_no_matches() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "hello world").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "nonexistent" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("No matches found"));
}

#[tokio::test]
async fn test_grep_with_matches() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "hello foo world").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "foo" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("foo"));
}

#[tokio::test]
async fn test_grep_case_insensitive() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "Hello FOO World").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({
        "pattern": "foo",
        "case_insensitive": true
    });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("FOO"));
}

#[tokio::test]
async fn test_grep_with_glob_filter() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.rs"), "foo content").await.unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "foo content").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({
        "pattern": "foo",
        "glob": "*.rs"
    });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("test.rs"));
    assert!(!result.content.contains("test.txt"));
}

#[tokio::test]
async fn test_grep_invalid_regex() {
    let temp = TempDir::new().unwrap();
    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "[invalid" });
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[test]
fn test_default_context() {
    assert_eq!(default_context(), 0);
}

#[test]
fn test_grep_params_parsing() {
    let json = serde_json::json!({
        "pattern": "foo"
    });
    let params: GrepParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.pattern, "foo");
    assert!(params.path.is_none());
    assert!(params.glob.is_none());
    assert_eq!(params.context, 0);
    assert!(!params.case_insensitive);
}

#[test]
fn test_grep_params_full() {
    let json = serde_json::json!({
        "pattern": "test",
        "path": "/src",
        "glob": "*.rs",
        "context": 3,
        "case_insensitive": true
    });
    let params: GrepParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, Some("/src".to_string()));
    assert_eq!(params.glob, Some("*.rs".to_string()));
    assert_eq!(params.context, 3);
    assert!(params.case_insensitive);
}

#[test]
fn test_grep_tool_risk_level() {
    let tool = GrepTool::new();
    assert_eq!(tool.definition().risk_level, RiskLevel::Low);
}

#[test]
fn test_search_file_multiple_matches() {
    let content = "foo line 1\nbar line 2\nfoo line 3\nbaz line 4\nfoo line 5";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 0);
    assert_eq!(matches.len(), 3);
}

#[test]
fn test_search_file_context_at_start() {
    let content = "foo bar\nline 2\nline 3";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 2);
    assert_eq!(matches.len(), 1);
    // Should include lines up to context limit
    assert!(matches[0].contains("foo bar"));
}

#[test]
fn test_search_file_context_at_end() {
    let content = "line 1\nline 2\nfoo bar";
    let regex = Regex::new("foo").unwrap();
    let matches = search_file(content, &regex, 2);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].contains("foo bar"));
}

#[test]
fn test_matches_glob_various_patterns() {
    assert!(matches_glob(std::path::Path::new("test.txt"), "*.txt"));
    assert!(matches_glob(std::path::Path::new("foo.bar.txt"), "*.txt"));
    assert!(matches_glob(std::path::Path::new("test.rs"), "test.*"));
    assert!(!matches_glob(std::path::Path::new("test.rs"), "*.py"));
}

#[test]
fn test_matches_glob_empty_filename() {
    assert!(!matches_glob(std::path::Path::new(""), "*.rs"));
}

#[tokio::test]
async fn test_grep_with_path() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("sub");
    tokio::fs::create_dir(&subdir).await.unwrap();
    tokio::fs::write(subdir.join("test.txt"), "foo content").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({
        "pattern": "foo",
        "path": subdir.to_string_lossy()
    });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("test.txt"));
}

#[tokio::test]
async fn test_grep_invalid_params() {
    let temp = TempDir::new().unwrap();
    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({});
    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_grep_with_context() {
    let temp = TempDir::new().unwrap();
    tokio::fs::write(temp.path().join("test.txt"), "line 1\nline 2\nfoo\nline 4\nline 5").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({
        "pattern": "foo",
        "context": 1
    });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("line 2"));
    assert!(result.content.contains("foo"));
    assert!(result.content.contains("line 4"));
}

#[tokio::test]
async fn test_grep_in_subdirectories() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("deep/nested");
    tokio::fs::create_dir_all(&subdir).await.unwrap();
    tokio::fs::write(subdir.join("file.txt"), "target content").await.unwrap();

    let tool = GrepTool::new();
    let ctx = ToolContext::new("test", temp.path().to_path_buf());
    let params = serde_json::json!({ "pattern": "target" });
    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("file.txt"));
}
