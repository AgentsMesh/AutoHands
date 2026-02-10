//! Navigation tools: navigate, back, forward, refresh, get URL.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::BrowserManager;

use super::default_timeout;

// ============================================================================
// Navigate Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct NavigateParams {
    pub page_id: String,
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

/// Navigate to URL tool.
pub struct NavigateTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl NavigateTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_navigate",
                "Browser Navigate",
                "Navigate a browser page to a URL",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for NavigateTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: NavigateParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .navigate(&params.page_id, &params.url)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Navigated {} to {}", params.page_id, params.url);
        Ok(ToolResult::success(format!("Navigated to {}", params.url)))
    }
}

// ============================================================================
// Back/Forward/Refresh Tools
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct NavigationParams {
    pub page_id: String,
}

/// Go back tool.
pub struct BackTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl BackTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_back",
                "Browser Back",
                "Go back to the previous page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for BackTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: NavigationParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .go_back(&params.page_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success("Navigated back"))
    }
}

/// Go forward tool.
pub struct ForwardTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ForwardTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_forward",
                "Browser Forward",
                "Go forward to the next page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ForwardTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: NavigationParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .go_forward(&params.page_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success("Navigated forward"))
    }
}

/// Refresh page tool.
pub struct RefreshTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl RefreshTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_refresh",
                "Browser Refresh",
                "Refresh the current page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for RefreshTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: NavigationParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .reload(&params.page_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success("Page refreshed"))
    }
}

// ============================================================================
// Get URL Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetUrlParams {
    pub page_id: String,
}

/// Get current URL tool.
pub struct GetUrlTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl GetUrlTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_get_url",
                "Browser Get URL",
                "Get the current URL of a page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for GetUrlTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GetUrlParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let url = self
            .manager
            .get_url(&params.page_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(url))
    }
}
