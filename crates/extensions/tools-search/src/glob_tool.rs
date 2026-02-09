//! Glob pattern matching tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct GlobParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

/// Glob pattern matching tool.
pub struct GlobTool {
    definition: ToolDefinition,
}

impl GlobTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., **/*.rs, src/**/*.ts)"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in"
                }
            },
            "required": ["pattern"]
        });

        Self {
            definition: ToolDefinition::new("glob", "Glob Search", "Find files matching a pattern")
                .with_parameters_schema(schema)
                .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GlobParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let base_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.work_dir.clone());

        let full_pattern = base_path.join(&params.pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let mut matches = Vec::new();
        for entry in glob::glob(&pattern_str)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?
        {
            match entry {
                Ok(path) => {
                    let relative = path
                        .strip_prefix(&base_path)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    matches.push(relative);
                }
                Err(e) => {
                    tracing::warn!("Glob error: {}", e);
                }
            }
        }

        if matches.is_empty() {
            Ok(ToolResult::success("No files found matching pattern"))
        } else {
            Ok(ToolResult::success(format!(
                "Found {} files:\n{}",
                matches.len(),
                matches.join("\n")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
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
}
