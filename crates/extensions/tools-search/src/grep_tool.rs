//! Content search (grep) tool.

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct GrepParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default = "default_context")]
    context: usize,
    #[serde(default)]
    case_insensitive: bool,
}

fn default_context() -> usize {
    0
}

/// Content search tool.
pub struct GrepTool {
    definition: ToolDefinition,
}

impl GrepTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "glob": {
                    "type": "string",
                    "description": "File pattern filter (e.g., *.rs)"
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context around matches"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                }
            },
            "required": ["pattern"]
        });

        Self {
            definition: ToolDefinition::new("grep", "Content Search", "Search file contents")
                .with_parameters_schema(schema)
                .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GrepParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let pattern = if params.case_insensitive {
            format!("(?i){}", params.pattern)
        } else {
            params.pattern.clone()
        };

        let regex = Regex::new(&pattern)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid regex: {}", e)))?;

        let search_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.work_dir.clone());

        let mut results = Vec::new();
        let glob_pattern = params.glob.as_deref();

        for entry in WalkDir::new(&search_path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            if let Some(glob) = glob_pattern {
                if !matches_glob(entry.path(), glob) {
                    continue;
                }
            }

            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                let matches = search_file(&content, &regex, params.context);
                if !matches.is_empty() {
                    let rel_path = entry.path()
                        .strip_prefix(&search_path)
                        .unwrap_or(entry.path());
                    results.push(format!("{}:\n{}", rel_path.display(), matches.join("\n")));
                }
            }
        }

        if results.is_empty() {
            Ok(ToolResult::success("No matches found"))
        } else {
            Ok(ToolResult::success(results.join("\n\n")))
        }
    }
}

fn matches_glob(path: &std::path::Path, pattern: &str) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    glob::Pattern::new(pattern)
        .map(|p| p.matches(file_name))
        .unwrap_or(false)
}

fn search_file(content: &str, regex: &Regex, context: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            let start = i.saturating_sub(context);
            let end = (i + context + 1).min(lines.len());
            let snippet: Vec<String> = lines[start..end]
                .iter()
                .enumerate()
                .map(|(j, l)| format!("{:>4}: {}", start + j + 1, l))
                .collect();
            matches.push(snippet.join("\n"));
        }
    }

    matches
}

#[cfg(test)]
mod tests {
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
}
