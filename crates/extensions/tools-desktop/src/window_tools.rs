//! Window management tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::window::WindowController;

// Helper to run blocking code
async fn run_blocking<F, T>(f: F) -> Result<T, ToolError>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        .map_err(ToolError::ExecutionFailed)
}

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

// ============================================================================
// Window Close Tool
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_list_tool_definition() {
        let tool = WindowListTool::new();
        assert_eq!(tool.definition().id, "desktop_window_list");
    }

    #[test]
    fn test_window_focus_tool_definition() {
        let tool = WindowFocusTool::new();
        assert_eq!(tool.definition().id, "desktop_window_focus");
    }

    #[test]
    fn test_window_move_tool_definition() {
        let tool = WindowMoveTool::new();
        assert_eq!(tool.definition().id, "desktop_window_move");
    }

    #[test]
    fn test_window_resize_tool_definition() {
        let tool = WindowResizeTool::new();
        assert_eq!(tool.definition().id, "desktop_window_resize");
    }

    #[test]
    fn test_window_minimize_tool_definition() {
        let tool = WindowMinimizeTool::new();
        assert_eq!(tool.definition().id, "desktop_window_minimize");
    }

    #[test]
    fn test_window_maximize_tool_definition() {
        let tool = WindowMaximizeTool::new();
        assert_eq!(tool.definition().id, "desktop_window_maximize");
    }

    #[test]
    fn test_window_close_tool_definition() {
        let tool = WindowCloseTool::new();
        assert_eq!(tool.definition().id, "desktop_window_close");
    }

    #[test]
    fn test_window_focus_params() {
        let json = serde_json::json!({"id": 123});
        let params: WindowFocusParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.id, 123);
    }

    #[test]
    fn test_window_move_params() {
        let json = serde_json::json!({"id": 123, "x": 100, "y": 200});
        let params: WindowMoveParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.id, 123);
        assert_eq!(params.x, 100);
        assert_eq!(params.y, 200);
    }

    #[test]
    fn test_window_resize_params() {
        let json = serde_json::json!({"id": 123, "width": 800, "height": 600});
        let params: WindowResizeParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.id, 123);
        assert_eq!(params.width, 800);
        assert_eq!(params.height, 600);
    }

    #[test]
    fn test_tools_default_impl() {
        let _ = WindowListTool::default();
        let _ = WindowFocusTool::default();
        let _ = WindowMoveTool::default();
        let _ = WindowResizeTool::default();
        let _ = WindowMinimizeTool::default();
        let _ = WindowMaximizeTool::default();
        let _ = WindowCloseTool::default();
    }
}
