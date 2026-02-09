//! Browser automation tools.

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
// Navigate Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct NavigateParams {
    pub page_id: String,
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
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
// Click Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ClickParams {
    pub page_id: String,
    pub selector: String,
}

/// Click element tool.
pub struct ClickTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ClickTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_click",
                "Browser Click",
                "Click an element on the page using CSS selector",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ClickTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ClickParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .click_selector(&params.page_id, &params.selector)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Clicked {}", params.selector);
        Ok(ToolResult::success(format!("Clicked {}", params.selector)))
    }
}

// ============================================================================
// Type Text Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TypeTextParams {
    pub page_id: String,
    pub selector: String,
    pub text: String,
    #[serde(default)]
    pub clear_first: bool,
}

/// Type text into element tool.
pub struct TypeTextTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl TypeTextTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_type",
                "Browser Type",
                "Type text into an input element",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for TypeTextTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: TypeTextParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // Fill the input field (Playwright's fill clears first by default)
        self.manager
            .fill(&params.page_id, &params.selector, &params.text)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Typed into {}", params.selector);
        Ok(ToolResult::success(format!(
            "Typed '{}' into {}",
            params.text, params.selector
        )))
    }
}

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

fn default_content_type() -> String {
    "text".to_string()
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
// Wait For Selector Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WaitForParams {
    pub page_id: String,
    pub selector: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

/// Wait for element tool.
pub struct WaitForTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl WaitForTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_wait_for",
                "Browser Wait For",
                "Wait for an element to appear on the page",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for WaitForTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WaitForParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .wait_for_selector(&params.page_id, &params.selector, Some(params.timeout_ms as u32))
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(format!(
            "Element {} found",
            params.selector
        )))
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

// ============================================================================
// Scroll Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ScrollParams {
    pub page_id: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    /// If selector is provided, scroll to that element
    pub selector: Option<String>,
}

/// Scroll page tool.
pub struct ScrollTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl ScrollTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_scroll",
                "Browser Scroll",
                "Scroll the page by x,y pixels or scroll to an element",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for ScrollTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ScrollParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        if let Some(ref selector) = params.selector {
            // Scroll to element
            let script = format!(
                r#"document.querySelector('{}')?.scrollIntoView({{behavior: 'smooth', block: 'center'}})"#,
                selector.replace('\'', "\\'")
            );
            self.manager
                .evaluate(&params.page_id, &script)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            Ok(ToolResult::success(format!("Scrolled to {}", selector)))
        } else {
            // Scroll by offset
            self.manager
                .scroll(&params.page_id, params.x as f64, params.y as f64)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            Ok(ToolResult::success(format!(
                "Scrolled by ({}, {})",
                params.x, params.y
            )))
        }
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

// ============================================================================
// Press Key Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PressKeyParams {
    pub page_id: String,
    /// Key name like "Enter", "Tab", "Escape", "ArrowDown", etc.
    pub key: String,
}

/// Press keyboard key tool.
pub struct PressKeyTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
}

impl PressKeyTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "browser_press_key",
                "Browser Press Key",
                "Press a keyboard key (Enter, Tab, Escape, ArrowDown, etc.)",
            ),
            manager,
        }
    }
}

#[async_trait]
impl Tool for PressKeyTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: PressKeyParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .press_key(&params.page_id, &params.key)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Pressed key {}", params.key);
        Ok(ToolResult::success(format!("Pressed {}", params.key)))
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
// Get DOM Tool (Browser-Use Style)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetDomParams {
    pub page_id: String,
    /// Return compact LLM-friendly format instead of full JSON
    #[serde(default = "default_compact")]
    pub compact: bool,
}

fn default_compact() -> bool {
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30000);
    }

    #[test]
    fn test_default_content_type() {
        assert_eq!(default_content_type(), "text");
    }

    #[test]
    fn test_navigate_params() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "url": "https://example.com"
        });
        let params: NavigateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.url, "https://example.com");
        assert_eq!(params.timeout_ms, 30000);
    }

    #[test]
    fn test_navigate_params_with_timeout() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "url": "https://example.com",
            "timeout_ms": 60000
        });
        let params: NavigateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.timeout_ms, 60000);
    }

    #[test]
    fn test_click_params() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "#button"
        });
        let params: ClickParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.selector, "#button");
    }

    #[test]
    fn test_type_params() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "input",
            "text": "hello"
        });
        let params: TypeTextParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.selector, "input");
        assert_eq!(params.text, "hello");
        assert!(!params.clear_first);
    }

    #[test]
    fn test_type_params_with_clear() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "input",
            "text": "hello",
            "clear_first": true
        });
        let params: TypeTextParams = serde_json::from_value(json).unwrap();
        assert!(params.clear_first);
    }

    #[test]
    fn test_screenshot_params() {
        let json = serde_json::json!({
            "page_id": "page_1"
        });
        let params: ScreenshotParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert!(!params.full_page);
        assert!(params.selector.is_none());
    }

    #[test]
    fn test_screenshot_params_full() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "full_page": true,
            "selector": "#element"
        });
        let params: ScreenshotParams = serde_json::from_value(json).unwrap();
        assert!(params.full_page);
        assert_eq!(params.selector, Some("#element".to_string()));
    }

    #[test]
    fn test_get_content_params() {
        let json = serde_json::json!({
            "page_id": "page_1"
        });
        let params: GetContentParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert!(params.selector.is_none());
        assert_eq!(params.content_type, "text");
    }

    #[test]
    fn test_get_content_params_html() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "div.content",
            "content_type": "html"
        });
        let params: GetContentParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.selector, Some("div.content".to_string()));
        assert_eq!(params.content_type, "html");
    }

    #[test]
    fn test_execute_js_params() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "script": "return document.title"
        });
        let params: ExecuteJsParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.script, "return document.title");
    }

    #[test]
    fn test_wait_for_params() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "#loading"
        });
        let params: WaitForParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.selector, "#loading");
        assert_eq!(params.timeout_ms, 30000);
    }

    #[test]
    fn test_wait_for_params_with_timeout() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "selector": "#loading",
            "timeout_ms": 5000
        });
        let params: WaitForParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.timeout_ms, 5000);
    }

    #[test]
    fn test_screenshot_result_serialize() {
        let result = ScreenshotResult {
            base64: "iVBORw0KGgo=".to_string(),
            width: 1920,
            height: 1080,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("iVBORw0KGgo="));
        assert!(json.contains("1920"));
        assert!(json.contains("1080"));
    }

    #[test]
    fn test_get_dom_params() {
        let json = serde_json::json!({
            "page_id": "page_1"
        });
        let params: GetDomParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert!(params.compact); // default
    }

    #[test]
    fn test_get_dom_params_full() {
        let json = serde_json::json!({
            "page_id": "page_1",
            "compact": false
        });
        let params: GetDomParams = serde_json::from_value(json).unwrap();
        assert!(!params.compact);
    }
}
