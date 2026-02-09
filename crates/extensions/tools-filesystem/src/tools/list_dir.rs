//! List directory tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for list_directory tool.
#[derive(Debug, Deserialize)]
struct ListDirectoryParams {
    /// Path to the directory to list.
    path: String,
    /// Maximum depth to recurse (default: 1).
    #[serde(default = "default_depth")]
    depth: usize,
}

fn default_depth() -> usize {
    1
}

/// List directory tool implementation.
pub struct ListDirectoryTool {
    definition: ToolDefinition,
}

impl ListDirectoryTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum depth to recurse (default: 1)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "list_directory",
                "List Directory",
                "List contents of a directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for ListDirectoryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListDirectoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ListDirectoryParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if !path.exists() {
            return Err(ToolError::ResourceNotFound(format!(
                "Directory not found: {}",
                path.display()
            )));
        }

        if !path.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(&path)
            .max_depth(params.depth)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(&path)
                .unwrap_or(entry.path());

            if relative.as_os_str().is_empty() {
                continue;
            }

            let prefix = if entry.file_type().is_dir() {
                "📁"
            } else {
                "📄"
            };

            entries.push(format!("{} {}", prefix, relative.display()));
        }

        Ok(ToolResult::success(entries.join("\n")))
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
        let tool = ListDirectoryTool::new();
        assert_eq!(tool.definition().id, "list_directory");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_default() {
        let tool = ListDirectoryTool::default();
        assert_eq!(tool.definition().id, "list_directory");
    }

    #[test]
    fn test_default_depth() {
        assert_eq!(default_depth(), 1);
    }

    #[tokio::test]
    async fn test_list_directory_success() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": temp_dir.path().to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("file1.txt"));
        assert!(result.content.contains("file2.txt"));
        assert!(result.content.contains("subdir"));
    }

    #[tokio::test]
    async fn test_list_directory_with_depth() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("a/b/c")).unwrap();
        std::fs::write(temp_dir.path().join("a/file.txt"), "").unwrap();
        std::fs::write(temp_dir.path().join("a/b/nested.txt"), "").unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": temp_dir.path().to_str().unwrap(),
            "depth": 3
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("file.txt"));
        assert!(result.content.contains("nested.txt"));
    }

    #[tokio::test]
    async fn test_list_directory_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "/nonexistent/directory"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ResourceNotFound(_) => {}
            e => panic!("Expected ResourceNotFound, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_list_directory_not_a_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "content").unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => assert!(msg.contains("Not a directory")),
            e => panic!("Expected ExecutionFailed, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_list_directory_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("subdir/file.txt"), "").unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": "subdir"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("file.txt"));
    }

    #[test]
    fn test_list_directory_tool_name() {
        let tool = ListDirectoryTool::new();
        assert_eq!(tool.definition().name, "List Directory");
    }

    #[test]
    fn test_list_directory_params_defaults() {
        let json = serde_json::json!({
            "path": "/tmp"
        });
        let params: ListDirectoryParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, "/tmp");
        assert_eq!(params.depth, 1);
    }

    #[test]
    fn test_list_directory_params_custom_depth() {
        let json = serde_json::json!({
            "path": "/tmp",
            "depth": 5
        });
        let params: ListDirectoryParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.depth, 5);
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
    async fn test_list_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let empty_dir = temp_dir.path().join("empty");
        std::fs::create_dir(&empty_dir).unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": empty_dir.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        assert!(result.content.is_empty());
    }

    #[tokio::test]
    async fn test_list_directory_invalid_params() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "invalid": "params"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_directory_depth_zero() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "").unwrap();

        let tool = ListDirectoryTool::new();
        let ctx = create_test_context(temp_dir.path().to_path_buf());
        let params = serde_json::json!({
            "path": temp_dir.path().to_str().unwrap(),
            "depth": 0
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
        // depth 0 means only the root directory itself
        assert!(result.content.is_empty());
    }
}
