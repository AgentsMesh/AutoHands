//! Read file tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for read_file tool.
#[derive(Debug, Deserialize)]
struct ReadFileParams {
    /// Path to the file to read.
    path: String,
    /// Starting line number (1-based, optional).
    #[serde(default)]
    offset: Option<usize>,
    /// Number of lines to read (optional).
    #[serde(default)]
    limit: Option<usize>,
}

/// Read file tool implementation.
pub struct ReadFileTool {
    definition: ToolDefinition,
}

impl ReadFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-based)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of lines to read"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new("read_file", "Read File", "Read contents of a file")
                .with_parameters_schema(schema)
                .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ReadFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&path).await?;

        // Apply offset and limit
        let lines: Vec<&str> = content.lines().collect();
        let offset = params.offset.unwrap_or(1).saturating_sub(1);
        let limit = params.limit.unwrap_or(lines.len());

        let selected: Vec<String> = lines
            .iter()
            .skip(offset)
            .take(limit)
            .enumerate()
            .map(|(i, line)| format!("{:>6}→{}", offset + i + 1, line))
            .collect();

        Ok(ToolResult::success(selected.join("\n")))
    }
}

fn resolve_path(path: &str, work_dir: &PathBuf) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        work_dir.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_context(work_dir: PathBuf) -> ToolContext {
        ToolContext::new("test", work_dir)
    }

    #[test]
    fn test_tool_definition() {
        let tool = ReadFileTool::new();
        assert_eq!(tool.definition().id, "read_file");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_default() {
        let tool = ReadFileTool::default();
        assert_eq!(tool.definition().id, "read_file");
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("line1"));
        assert!(result.content.contains("line2"));
    }

    #[tokio::test]
    async fn test_read_file_with_offset_and_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "offset": 2,
            "limit": 2
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("line2"));
        assert!(result.content.contains("line3"));
        assert!(!result.content.contains("line1"));
        assert!(!result.content.contains("line4"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "/nonexistent/file.txt"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ResourceNotFound(_) => {}
            e => panic!("Expected ResourceNotFound, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_read_file_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir/test.txt");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "content").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "subdir/test.txt"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("content"));
    }

    #[tokio::test]
    async fn test_read_file_invalid_params() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({});

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
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

    #[test]
    fn test_read_file_params_defaults() {
        let json = serde_json::json!({
            "path": "test.txt"
        });
        let params: ReadFileParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, "test.txt");
        assert!(params.offset.is_none());
        assert!(params.limit.is_none());
    }

    #[test]
    fn test_read_file_params_with_options() {
        let json = serde_json::json!({
            "path": "test.txt",
            "offset": 10,
            "limit": 20
        });
        let params: ReadFileParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.offset, Some(10));
        assert_eq!(params.limit, Some(20));
    }

    #[test]
    fn test_read_file_tool_name() {
        let tool = ReadFileTool::new();
        assert_eq!(tool.definition().name, "Read File");
    }

    #[tokio::test]
    async fn test_read_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        std::fs::write(&file_path, "").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.is_empty());
    }

    #[tokio::test]
    async fn test_read_file_offset_beyond_end() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "offset": 100
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.is_empty());
    }

    #[tokio::test]
    async fn test_read_file_limit_zero() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2").unwrap();

        let tool = ReadFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "limit": 0
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.is_empty());
    }
}
