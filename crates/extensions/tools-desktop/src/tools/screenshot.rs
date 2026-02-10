//! Screenshot and screen info tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::screenshot;

use super::run_blocking;

#[derive(Debug, Deserialize)]
pub struct ScreenshotParams {
    pub region: Option<RegionParams>,
}

#[derive(Debug, Deserialize)]
pub struct RegionParams {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Take a desktop screenshot.
pub struct DesktopScreenshotTool {
    definition: ToolDefinition,
}

impl DesktopScreenshotTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_screenshot",
                "Desktop Screenshot",
                "Take a screenshot of the entire screen or a region",
            ),
        }
    }
}

impl Default for DesktopScreenshotTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DesktopScreenshotTool {
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

        let screenshot = run_blocking(move || {
            let result = if let Some(region) = params.region {
                screenshot::capture_region(region.x, region.y, region.width, region.height)
            } else {
                screenshot::capture_screen()
            };
            result.map_err(|e| e.to_string())
        })
        .await?;

        debug!(
            "Screenshot captured: {}x{}, {} bytes",
            screenshot.width,
            screenshot.height,
            screenshot.data.len()
        );

        Ok(ToolResult::success(format!(
            "Screenshot captured: {}x{}",
            screenshot.width, screenshot.height
        ))
        .with_metadata("base64", serde_json::json!(screenshot.to_base64()))
        .with_metadata("width", serde_json::json!(screenshot.width))
        .with_metadata("height", serde_json::json!(screenshot.height)))
    }
}

/// Get screen information.
pub struct ScreenInfoTool {
    definition: ToolDefinition,
}

impl ScreenInfoTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_screen_info",
                "Desktop Screen Info",
                "Get information about screens/monitors",
            ),
        }
    }
}

impl Default for ScreenInfoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ScreenInfoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let monitors = run_blocking(|| screenshot::list_monitors().map_err(|e| e.to_string()))
            .await?;

        let result = serde_json::to_string_pretty(&monitors)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(result))
    }
}
