//! Content retrieval tools: screenshot, get content, execute JS, get DOM.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::BrowserManager;

use super::{default_compact, default_content_type};

// ============================================================================
// Screenshot Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ScreenshotParams {
    pub page_id: String,
    #[serde(default)]
    pub full_page: bool,
    pub selector: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScreenshotResult {
    pub base64: String,
    pub width: u32,
    pub height: u32,
}

/// Take screenshot tool.
pub struct ScreenshotTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ScreenshotTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_screenshot",
                "Browser Screenshot",
                "Take a screenshot of the page or element",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ScreenshotTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ScreenshotParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // TODO: Support selector-based screenshots
        let base64 = self
            .manager
            .screenshot(&params.page_id, params.full_page)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Screenshot taken");

        // Estimate size from base64 length (not exact but reasonable)
        let estimated_bytes = base64.len() * 3 / 4;
        Ok(ToolResult::success(format!(
            "Screenshot captured (~{} bytes)",
            estimated_bytes
        ))
        .with_metadata("base64", serde_json::json!(base64)))
    }
}

// ============================================================================
// Get Content Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetContentParams {
    pub page_id: String,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
}

/// Get page content tool.
pub struct GetContentTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl GetContentTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_get_content",
                "Browser Get Content",
                "Get text or HTML content from page or element",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for GetContentTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GetContentParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let content = if params.content_type == "html" {
            self.manager
                .get_content(&params.page_id)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        } else {
            // Get text content via JavaScript
            let script = if let Some(ref selector) = params.selector {
                format!(
                    "document.querySelector('{}')?.innerText || ''",
                    selector.replace('\'', "\\'")
                )
            } else {
                "document.body.innerText".to_string()
            };

            let result = self
                .manager
                .evaluate(&params.page_id, &script)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            result.as_str().unwrap_or("").to_string()
        };

        Ok(ToolResult::success(content))
    }
}

// ============================================================================
// Execute JavaScript Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExecuteJsParams {
    pub page_id: String,
    pub script: String,
}

/// Execute JavaScript tool.
pub struct ExecuteJsTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ExecuteJsTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_execute_js",
                "Browser Execute JavaScript",
                "Execute JavaScript code on the page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ExecuteJsTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ExecuteJsParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let result = self
            .manager
            .evaluate(&params.page_id, &params.script)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Executed JavaScript");
        Ok(ToolResult::success(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        ))
    }
}

// ============================================================================
// Get DOM Tool (Browser-Use Style)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetDomParams {
    pub page_id: String,
    /// Return compact LLM-friendly format instead of full JSON
    #[serde(default = "default_compact")]
    pub compact: bool,
}

/// Get enhanced DOM tree tool with clickability analysis.
pub struct GetDomTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl GetDomTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "browser_get_dom",
            "Browser Get DOM",
            "Get enhanced DOM tree with interactive elements and clickability scores. Use compact=true for LLM-friendly output.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "The page ID to analyze"
                },
                "compact": {
                    "type": "boolean",
                    "description": "Return compact LLM-friendly format (default: true)",
                    "default": true
                }
            },
            "required": ["page_id"]
        }));
        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for GetDomTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GetDomParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        if params.compact {
            // Return LLM-friendly format
            let output = self
                .manager
                .get_page_for_llm(&params.page_id)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            Ok(ToolResult::success(output))
        } else {
            // Return full DOM tree as JSON
            let dom_tree = self
                .manager
                .get_dom_tree(&params.page_id)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            Ok(ToolResult::success(
                serde_json::to_string_pretty(&dom_tree).unwrap_or_default(),
            ))
        }
    }
}
