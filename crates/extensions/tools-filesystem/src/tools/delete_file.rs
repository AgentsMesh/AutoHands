//! Delete file tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for delete_file tool.
#[derive(Debug, Deserialize)]
struct DeleteFileParams {
    /// Path to the file or directory to delete.
    path: String,
    /// Delete directories recursively.
    #[serde(default)]
    recursive: bool,
}

/// Delete file tool implementation.
pub struct DeleteFileTool {
    definition: ToolDefinition,
}

impl DeleteFileTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Delete directories recursively (required for non-empty directories)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "delete_file",
                "Delete File",
                "Delete a file or directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::High),
        }
    }
}

impl Default for DeleteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DeleteFileTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: DeleteFileParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "Path not found: {}",
                path.display()
            )));
        }

        let metadata = tokio::fs::metadata(&path).await?;

        if metadata.is_dir() {
            if params.recursive {
                tokio::fs::remove_dir_all(&path).await?;
            } else {
                tokio::fs::remove_dir(&path).await.map_err(|e| {
                    if e.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                        ToolError::ExecutionFailed(
                            "Directory not empty. Use recursive=true to delete non-empty directories.".to_string()
                        )
                    } else {
                        ToolError::from(e)
                    }
                })?;
            }
            Ok(ToolResult::success(format!(
                "Deleted directory: {}",
                path.display()
            )))
        } else {
            tokio::fs::remove_file(&path).await?;
            Ok(ToolResult::success(format!(
                "Deleted file: {}",
                path.display()
            )))
        }
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

    #[tokio::test]
    async fn test_delete_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();

        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "test.txt"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Deleted file"));
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_empty_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("empty_dir");
        tokio::fs::create_dir(&dir_path).await.unwrap();

        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "empty_dir"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Deleted directory"));
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_delete_directory_recursive() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("dir");
        tokio::fs::create_dir(&dir_path).await.unwrap();
        tokio::fs::write(dir_path.join("file.txt"), "content").await.unwrap();

        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "dir",
            "recursive": true
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Deleted directory"));
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let temp = TempDir::new().unwrap();
        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "nonexistent.txt"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_file_tool_default() {
        let tool = DeleteFileTool::default();
        assert_eq!(tool.definition().id, "delete_file");
    }

    #[test]
    fn test_delete_file_tool_definition() {
        let tool = DeleteFileTool::new();
        assert_eq!(tool.definition().name, "Delete File");
        assert_eq!(tool.definition().risk_level, RiskLevel::High);
    }

    #[test]
    fn test_delete_file_params_parsing() {
        let json = serde_json::json!({
            "path": "/tmp/test.txt"
        });
        let params: DeleteFileParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, "/tmp/test.txt");
        assert!(!params.recursive);
    }

    #[test]
    fn test_delete_file_params_with_recursive() {
        let json = serde_json::json!({
            "path": "/tmp/dir",
            "recursive": true
        });
        let params: DeleteFileParams = serde_json::from_value(json).unwrap();
        assert!(params.recursive);
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
    async fn test_delete_non_empty_dir_without_recursive() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("non_empty_dir");
        tokio::fs::create_dir(&dir_path).await.unwrap();
        tokio::fs::write(dir_path.join("file.txt"), "content").await.unwrap();

        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "non_empty_dir",
            "recursive": false
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_file_with_absolute_path() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("absolute_test.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();

        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("/"));

        let params = serde_json::json!({
            "path": file_path.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Deleted file"));
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_invalid_params() {
        let temp = TempDir::new().unwrap();
        let tool = DeleteFileTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "invalid": "params"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }
}
