use super::*;
use tempfile::TempDir;

fn create_test_context(work_dir: std::path::PathBuf) -> ToolContext {
    ToolContext::new("test", work_dir)
}

#[test]
fn test_analyze_code_tool_creation() {
    let tool = AnalyzeCodeTool::new();
    assert_eq!(tool.definition().name, "Analyze Code");
    assert_eq!(tool.definition().id, "analyze_code");
}

#[test]
fn test_analyze_code_tool_default() {
    let tool = AnalyzeCodeTool::default();
    assert_eq!(tool.definition().id, "analyze_code");
}

#[test]
fn test_find_symbol_tool_creation() {
    let tool = FindSymbolTool::new();
    assert_eq!(tool.definition().name, "Find Symbol");
    assert_eq!(tool.definition().id, "find_symbol");
}

#[test]
fn test_find_symbol_tool_default() {
    let tool = FindSymbolTool::default();
    assert_eq!(tool.definition().id, "find_symbol");
}

#[test]
fn test_analyze_code_params() {
    let json = serde_json::json!({
        "path": "test.rs",
        "include_signatures": true
    });
    let params: AnalyzeCodeParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.path, "test.rs");
    assert!(params.include_signatures);
}

#[test]
fn test_analyze_code_params_defaults() {
    let json = serde_json::json!({
        "path": "test.rs"
    });
    let params: AnalyzeCodeParams = serde_json::from_value(json).unwrap();
    assert!(!params.include_signatures);
}

#[test]
fn test_find_symbol_params() {
    let json = serde_json::json!({
        "symbol": "MyFunction",
        "path": "src/"
    });
    let params: FindSymbolParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.symbol, "MyFunction");
    assert!(!params.case_sensitive);
}

#[test]
fn test_find_symbol_params_case_sensitive() {
    let json = serde_json::json!({
        "symbol": "MyFunction",
        "path": "src/",
        "case_sensitive": true
    });
    let params: FindSymbolParams = serde_json::from_value(json).unwrap();
    assert!(params.case_sensitive);
}

#[test]
fn test_symbol_match_serialize() {
    let m = SymbolMatch {
        file: "test.rs".to_string(),
        line: 10,
        content: "fn test()".to_string(),
        element_type: Some("function".to_string()),
    };
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("test.rs"));
    assert!(json.contains("10"));
    assert!(json.contains("fn test()"));
    assert!(json.contains("function"));
}

#[test]
fn test_symbol_match_serialize_without_type() {
    let m = SymbolMatch {
        file: "test.py".to_string(),
        line: 5,
        content: "def func():".to_string(),
        element_type: None,
    };
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("test.py"));
    assert!(json.contains("def func():"));
}

#[test]
fn test_symbol_match_debug() {
    let m = SymbolMatch {
        file: "test.rs".to_string(),
        line: 1,
        content: "test".to_string(),
        element_type: None,
    };
    let debug_str = format!("{:?}", m);
    assert!(debug_str.contains("SymbolMatch"));
}

#[tokio::test]
async fn test_analyze_code_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = AnalyzeCodeTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "path": "nonexistent.rs"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_analyze_code_rust_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn main() { println!(\"Hello\"); }").unwrap();

    let tool = AnalyzeCodeTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "path": "test.rs"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("rust"));
}

#[tokio::test]
async fn test_find_symbol_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = FindSymbolTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "symbol": "test",
        "path": "nonexistent.rs"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_symbol_in_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn main() {\n    println!(\"Hello\");\n}\n\nfn helper() {}").unwrap();

    let tool = FindSymbolTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "symbol": "fn",
        "path": "test.rs"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("main"));
    assert!(result.content.contains("helper"));
}

#[tokio::test]
async fn test_find_symbol_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn MyFunction() {}").unwrap();

    let tool = FindSymbolTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "symbol": "myfunction",
        "path": "test.rs",
        "case_sensitive": false
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("MyFunction"));
}

#[tokio::test]
async fn test_find_symbol_case_sensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn MyFunction() {}").unwrap();

    let tool = FindSymbolTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "symbol": "myfunction",
        "path": "test.rs",
        "case_sensitive": true
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    // Should NOT find it because case doesn't match
    assert_eq!(result.content, "[]");
}

#[tokio::test]
async fn test_find_symbol_no_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn main() {}").unwrap();

    let tool = FindSymbolTool::new();
    let ctx = create_test_context(temp_dir.path().to_path_buf());

    let params = serde_json::json!({
        "symbol": "nonexistent_symbol",
        "path": "test.rs"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
    assert_eq!(result.content, "[]");
}
