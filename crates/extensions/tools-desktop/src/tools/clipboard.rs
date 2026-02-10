//! Clipboard tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::clipboard::ClipboardController;

use super::run_blocking;

// ============================================================================
// Clipboard Get Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ClipboardGetParams {
    #[serde(default = "default_content_type")]
    pub content_type: String,
}

fn default_content_type() -> String {
    "text".to_string()
}

/// Get clipboard content.
pub struct ClipboardGetTool {
    definition: ToolDefinition,
}

impl ClipboardGetTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_clipboard_get",
                "Desktop Clipboard Get",
                "Get content from the clipboard (text or image)",
            ),
        }
    }
}

impl Default for ClipboardGetTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ClipboardGetTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ClipboardGetParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        if params.content_type == "image" {
            let data = run_blocking(|| {
                let mut controller = ClipboardController::new().map_err(|e| e.to_string())?;
                controller.get_image().map_err(|e| e.to_string())
            })
            .await?;

            use base64::Engine;
            let base64 = base64::engine::general_purpose::STANDARD.encode(&data);

            Ok(ToolResult::success("Image from clipboard")
                .with_metadata("base64", serde_json::json!(base64)))
        } else {
            let text = run_blocking(|| {
                let mut controller = ClipboardController::new().map_err(|e| e.to_string())?;
                controller.get_text().map_err(|e| e.to_string())
            })
            .await?;

            Ok(ToolResult::success(text))
        }
    }
}

// ============================================================================
// Clipboard Set Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ClipboardSetParams {
    pub text: String,
}

/// Set clipboard content.
pub struct ClipboardSetTool {
    definition: ToolDefinition,
}

impl ClipboardSetTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_clipboard_set",
                "Desktop Clipboard Set",
                "Set text content to the clipboard",
            ),
        }
    }
}

impl Default for ClipboardSetTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ClipboardSetTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ClipboardSetParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        run_blocking(move || {
            let mut controller = ClipboardController::new().map_err(|e| e.to_string())?;
            controller.set_text(&params.text).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Set clipboard text");
        Ok(ToolResult::success("Clipboard updated"))
    }
}
