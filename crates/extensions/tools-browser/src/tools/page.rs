//! Page lifecycle tools: open, close, list.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::BrowserManager;

// ============================================================================
// Open Page Tool (creates new page and triggers lazy browser init)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OpenPageParams {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct OpenPageResult {
    pub page_id: String,
    pub url: String,
}

/// Open a new browser page tool.
///
/// This tool creates a new browser page and navigates to the specified URL.
/// The browser is lazily initialized when this tool is first called.
pub struct OpenPageTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl OpenPageTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "browser_open",
            "Browser Open",
            "Open a new browser page and navigate to URL. Returns page_id for subsequent operations.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                }
            },
            "required": ["url"]
        }));
        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for OpenPageTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: OpenPageParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // This will lazily initialize the browser if not already running
        let page_id = self
            .manager
            .new_page(&params.url)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Opened new page {}: {}", page_id, params.url);

        let result = OpenPageResult {
            page_id: page_id.clone(),
            url: params.url,
        };

        Ok(ToolResult::success(serde_json::to_string(&result).unwrap())
            .with_metadata("page_id", serde_json::json!(page_id)))
    }
}

// ============================================================================
// Close Page Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ClosePageParams {
    pub page_id: String,
}

/// Close a browser page tool.
pub struct ClosePageTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ClosePageTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_close",
                "Browser Close",
                "Close a browser page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ClosePageTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ClosePageParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .close_page(&params.page_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Closed page {}", params.page_id);
        Ok(ToolResult::success(format!("Closed page {}", params.page_id)))
    }
}

// ============================================================================
// List Pages Tool
// ============================================================================

/// List all open browser pages tool.
pub struct ListPagesTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ListPagesTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_list_pages",
                "Browser List Pages",
                "List all open browser pages",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ListPagesTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let pages = self.manager.list_pages().await;
        Ok(ToolResult::success(serde_json::to_string(&pages).unwrap()))
    }
}
