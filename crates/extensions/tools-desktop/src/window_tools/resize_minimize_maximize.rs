//! Window resize, minimize, and maximize tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::window::WindowController;

use super::run_blocking;

// ============================================================================
// Window Resize Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WindowResizeParams {
    /// Window ID to resize.
    pub id: u64,
    /// New width.
    pub width: u32,
    /// New height.
    pub height: u32,
}

/// Resize a window.
pub struct WindowResizeTool {
    definition: ToolDefinition,
}

impl WindowResizeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_resize",
                "Desktop Window Resize",
                "Resize a window to new dimensions",
            ),
        }
    }
}

impl Default for WindowResizeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowResizeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowResizeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;
        let width = params.width;
        let height = params.height;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller
                .resize_window(id, width, height)
                .map_err(|e| e.to_string())
        })
        .await?;

        debug!("Resized window {} to {}x{}", id, width, height);
        Ok(ToolResult::success(format!(
            "Resized window {} to {}x{}",
            id, width, height
        )))
    }
}

// ============================================================================
// Window Minimize Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WindowMinimizeParams {
    /// Window ID to minimize.
    pub id: u64,
}

/// Minimize a window.
pub struct WindowMinimizeTool {
    definition: ToolDefinition,
}

impl WindowMinimizeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_minimize",
                "Desktop Window Minimize",
                "Minimize a window",
            ),
        }
    }
}

impl Default for WindowMinimizeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowMinimizeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowMinimizeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.minimize_window(id).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Minimized window {}", id);
        Ok(ToolResult::success(format!("Minimized window {}", id)))
    }
}

// ============================================================================
// Window Maximize Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WindowMaximizeParams {
    /// Window ID to maximize.
    pub id: u64,
}

/// Maximize a window.
pub struct WindowMaximizeTool {
    definition: ToolDefinition,
}

impl WindowMaximizeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_maximize",
                "Desktop Window Maximize",
                "Maximize a window",
            ),
        }
    }
}

impl Default for WindowMaximizeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowMaximizeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowMaximizeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.maximize_window(id).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Maximized window {}", id);
        Ok(ToolResult::success(format!("Maximized window {}", id)))
    }
}
