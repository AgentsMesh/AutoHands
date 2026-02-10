//! Window list, focus, and move tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::window::WindowController;

use super::run_blocking;

// ============================================================================
// Window List Tool
// ============================================================================

/// List all windows.
pub struct WindowListTool {
    definition: ToolDefinition,
}

impl WindowListTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_list",
                "Desktop Window List",
                "List all visible windows with their IDs, titles, and positions",
            ),
        }
    }
}

impl Default for WindowListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowListTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let windows = run_blocking(|| {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.list_windows().map_err(|e| e.to_string())
        })
        .await?;

        let json = serde_json::to_string_pretty(&windows)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Listed {} windows", windows.len());
        Ok(ToolResult::success(json))
    }
}

// ============================================================================
// Window Focus Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WindowFocusParams {
    /// Window ID to focus.
    pub id: u64,
}

/// Focus a window.
pub struct WindowFocusTool {
    definition: ToolDefinition,
}

impl WindowFocusTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_focus",
                "Desktop Window Focus",
                "Focus a window by its ID",
            ),
        }
    }
}

impl Default for WindowFocusTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowFocusTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowFocusParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.focus_window(id).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Focused window {}", id);
        Ok(ToolResult::success(format!("Focused window {}", id)))
    }
}

// ============================================================================
// Window Move Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WindowMoveParams {
    /// Window ID to move.
    pub id: u64,
    /// New X position.
    pub x: i32,
    /// New Y position.
    pub y: i32,
}

/// Move a window.
pub struct WindowMoveTool {
    definition: ToolDefinition,
}

impl WindowMoveTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_window_move",
                "Desktop Window Move",
                "Move a window to a new position",
            ),
        }
    }
}

impl Default for WindowMoveTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WindowMoveTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WindowMoveParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let id = params.id;
        let x = params.x;
        let y = params.y;

        run_blocking(move || {
            let controller = WindowController::new().map_err(|e| e.to_string())?;
            controller.move_window(id, x, y).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Moved window {} to ({}, {})", id, x, y);
        Ok(ToolResult::success(format!(
            "Moved window {} to ({}, {})",
            id, x, y
        )))
    }
}
