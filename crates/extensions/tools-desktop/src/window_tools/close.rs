//! Window close tool.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::window::WindowController;

use super::run_blocking;

#[derive(Debug, Deserialize)]
pub struct WindowCloseParams {
    /// Window ID to close.
    pub id: u64,
}

/// Close a window.
pub struct WindowCloseTool {
    definition: ToolDefinition,
}

impl WindowCloseTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_close",
                "Desktop Window Close",
                "Close a window",
            ),
        }
    }
}

impl Default for WindowCloseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowCloseTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowCloseParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.close_window(id).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Closed window {}", id);
        Ok(ToolResult::success(format!("Closed window {}", id)))
    }
}
