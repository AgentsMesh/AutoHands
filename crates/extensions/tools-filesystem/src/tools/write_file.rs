//! Write file tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for write_file tool.
#[derive(Debug, Deserialize)]
struct WriteFileParams {
    /// Path to the file to write.
    path: String,
    /// Content to write.
    content: String,
}

/// Write file tool implementation.
pub struct WriteFileTool {
    definition: ToolDefinition,
}

impl WriteFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        });

        Self {
            definition: ToolDefinition::new(
                "write_file",
                "Write File",
                "Write content to a file (overwrites existing)",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WriteFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, &params.content).await?;

        Ok(ToolResult::success(format!(
            "Successfully wrote {} bytes to {}",
            params.content.len(),
            path.display()
        )))
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
        let tool = WriteFileTool::new();
        assert_eq!(tool.definition().id, "write_file");
        assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_default() {
        let tool = WriteFileTool::default();
        assert_eq!(tool.definition().id, "write_file");
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "Hello, World!"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("13 bytes"));

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("a/b/c/test.txt");

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "nested content"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_write_file_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "old content").unwrap();

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "new content"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn test_write_file_relative_path() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "relative.txt",
            "content": "relative content"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);

        let file_path = temp_dir.path().join("relative.txt");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_write_file_invalid_params() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "/test.txt"
            // missing content
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_write_file_tool_name() {
        let tool = WriteFileTool::new();
        assert_eq!(tool.definition().name, "Write File");
    }

    #[test]
    fn test_write_file_params_parsing() {
        let json = serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "hello world"
        });
        let params: WriteFileParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, "/tmp/test.txt");
        assert_eq!(params.content, "hello world");
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

    #[tokio::test]
    async fn test_write_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": ""
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("0 bytes"));
    }

    #[tokio::test]
    async fn test_write_file_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("unicode.txt");

        let tool = WriteFileTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "你好世界 🌍"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "你好世界 🌍");
    }
}
