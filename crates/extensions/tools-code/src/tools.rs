//! Code analysis tools.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::analyzer::{detect_language, FileAnalysis, PatternAnalyzer};

/// Parameters for code analysis.
#[derive(Debug, Deserialize)]
pub struct AnalyzeCodeParams {
    pub path: String,
    #[serde(default)]
    pub include_signatures: bool,
}

/// Code analyzer tool.
pub struct AnalyzeCodeTool {
    definition: ToolDefinition,
}

impl AnalyzeCodeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "analyze_code",
                "Analyze Code",
                "Analyze source code to extract functions, classes, structs, and other elements",
            ),
        }
    }
}

impl Default for AnalyzeCodeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AnalyzeCodeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AnalyzeCodeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let path = ctx.work_dir.join(&params.path);
        debug!("Analyzing code: {:?}", path);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                params.path
            )));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let language = detect_language(&path).unwrap_or_else(|| "unknown".to_string());

        let elements = if language == "rust" {
            PatternAnalyzer::analyze_rust(&content)
        } else {
            Vec::new()
        };

        let analysis = FileAnalysis {
            path: params.path,
            language,
            elements,
            imports: Vec::new(),
            line_count: content.lines().count(),
        };

        let result = serde_json::to_string_pretty(&analysis)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(result))
    }
}

/// Parameters for symbol search.
#[derive(Debug, Deserialize)]
pub struct FindSymbolParams {
    pub symbol: String,
    pub path: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

/// Symbol search result.
#[derive(Debug, Serialize)]
pub struct SymbolMatch {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub element_type: Option<String>,
}

/// Find symbol tool.
pub struct FindSymbolTool {
    definition: ToolDefinition,
}

impl FindSymbolTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "find_symbol",
                "Find Symbol",
                "Search for a symbol (function, class, variable) in the codebase",
            ),
        }
    }
}

impl Default for FindSymbolTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FindSymbolTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: FindSymbolParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let path = ctx.work_dir.join(&params.path);
        debug!("Finding symbol '{}' in {:?}", params.symbol, path);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path not found: {}",
                params.path
            )));
        }

        let mut matches = Vec::new();

        if path.is_file() {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            for (i, line) in content.lines().enumerate() {
                let found = if params.case_sensitive {
                    line.contains(&params.symbol)
                } else {
                    line.to_lowercase()
                        .contains(&params.symbol.to_lowercase())
                };

                if found {
                    matches.push(SymbolMatch {
                        file: params.path.clone(),
                        line: i + 1,
                        content: line.trim().to_string(),
                        element_type: None,
                    });
                }
            }
        }

        let result = serde_json::to_string_pretty(&matches)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(result))
    }
}

#[cfg(test)]
mod tests {
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
}
