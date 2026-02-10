//! User interaction tools: click, type, press key, scroll, wait for.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::BrowserManager;

use super::default_timeout;

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
