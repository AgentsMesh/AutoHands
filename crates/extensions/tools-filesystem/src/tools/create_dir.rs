//! Create directory tool.

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for create_directory tool.
#[derive(Debug, Deserialize)]
struct CreateDirParams {
    /// Path to the directory to create.
    path: String,
    /// Create parent directories if they don't exist.
    #[serde(default = "default_true")]
    parents: bool,
}

fn default_true() -> bool {
    true
}

/// Create directory tool implementation.
pub struct CreateDirectoryTool {
    definition: ToolDefinition,
}

impl CreateDirectoryTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to create"
                },
                "parents": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist (default: true)"
                }
            },
            "required": ["path"]
        });

        Self {
            definition: ToolDefinition::new(
                "create_directory",
                "Create Directory",
                "Create a new directory",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Medium),
        }
    }
}

impl Default for CreateDirectoryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: CreateDirParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let path = resolve_path(&params.path, &ctx.work_dir);

        if path.exists() {
            return Ok(ToolResult::success(format!(
                "Directory already exists: {}",
                path.display()
            )));
        }

        if params.parents {
            tokio::fs::create_dir_all(&path).await?;
        } else {
            tokio::fs::create_dir(&path).await?;
        }

        Ok(ToolResult::success(format!(
            "Created directory: {}",
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

    #[tokio::test]
    async fn test_create_directory() {
        let temp = TempDir::new().unwrap();
        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "new_dir"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Created directory"));
        assert!(temp.path().join("new_dir").exists());
    }

    #[tokio::test]
    async fn test_create_nested_directory() {
        let temp = TempDir::new().unwrap();
        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "a/b/c",
            "parents": true
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Created directory"));
        assert!(temp.path().join("a/b/c").exists());
    }

    #[tokio::test]
    async fn test_create_existing_directory() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join("existing")).unwrap();

        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "existing"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("already exists"));
    }

    #[test]
    fn test_create_directory_tool_default() {
        let tool = CreateDirectoryTool::default();
        assert_eq!(tool.definition().id, "create_directory");
    }

    #[test]
    fn test_create_directory_tool_definition() {
        let tool = CreateDirectoryTool::new();
        assert_eq!(tool.definition().name, "Create Directory");
        assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_create_dir_params_parsing() {
        let json = serde_json::json!({
            "path": "new_dir"
        });
        let params: CreateDirParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, "new_dir");
        assert!(params.parents); // default is true
    }

    #[test]
    fn test_create_dir_params_no_parents() {
        let json = serde_json::json!({
            "path": "new_dir",
            "parents": false
        });
        let params: CreateDirParams = serde_json::from_value(json).unwrap();
        assert!(!params.parents);
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
    async fn test_create_directory_no_parents() {
        let temp = TempDir::new().unwrap();
        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "single_dir",
            "parents": false
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Created directory"));
        assert!(temp.path().join("single_dir").exists());
    }

    #[tokio::test]
    async fn test_create_nested_without_parents_fails() {
        let temp = TempDir::new().unwrap();
        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "path": "a/b/c",
            "parents": false
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_directory_invalid_params() {
        let temp = TempDir::new().unwrap();
        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", temp.path().to_path_buf());

        let params = serde_json::json!({
            "invalid": "params"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_directory_absolute_path() {
        let temp = TempDir::new().unwrap();
        let abs_path = temp.path().join("abs_dir");

        let tool = CreateDirectoryTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("/"));

        let params = serde_json::json!({
            "path": abs_path.to_str().unwrap()
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("Created directory"));
        assert!(abs_path.exists());
    }
}
