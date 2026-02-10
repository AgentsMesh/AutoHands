//! Mouse control tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::input::{InputController, MouseButton};

use super::run_blocking;

// ============================================================================
// Mouse Move Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct MouseMoveParams {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub relative: bool,
}

/// Move the mouse cursor.
pub struct MouseMoveTool {
    definition: ToolDefinition,
}

impl MouseMoveTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_mouse_move",
                "Desktop Mouse Move",
                "Move the mouse cursor to a position (absolute or relative)",
            ),
        }
    }
}

impl Default for MouseMoveTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MouseMoveTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MouseMoveParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let x = params.x;
        let y = params.y;
        let relative = params.relative;

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;
            if relative {
                controller.mouse_move_relative(x, y)
            } else {
                controller.mouse_move(x, y)
            }
            .map_err(|e| e.to_string())
        })
        .await?;

        debug!("Mouse moved to ({}, {})", x, y);
        Ok(ToolResult::success(format!("Mouse moved to ({}, {})", x, y)))
    }
}

// ============================================================================
// Mouse Click Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct MouseClickParams {
    #[serde(default = "default_button")]
    pub button: MouseButton,
    #[serde(default)]
    pub double_click: bool,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

fn default_button() -> MouseButton {
    MouseButton::Left
}

/// Click the mouse.
pub struct MouseClickTool {
    definition: ToolDefinition,
}

impl MouseClickTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_mouse_click",
                "Desktop Mouse Click",
                "Click the mouse (left, right, or middle button)",
            ),
        }
    }
}

impl Default for MouseClickTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MouseClickTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MouseClickParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;

            if let (Some(x), Some(y)) = (params.x, params.y) {
                controller.mouse_move(x, y).map_err(|e| e.to_string())?;
            }

            if params.double_click {
                controller.mouse_double_click(params.button)
            } else {
                controller.mouse_click(params.button)
            }
            .map_err(|e| e.to_string())
        })
        .await?;

        debug!("Mouse clicked");
        Ok(ToolResult::success("Mouse clicked"))
    }
}

// ============================================================================
// Mouse Scroll Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct MouseScrollParams {
    pub delta: i32,
    #[serde(default)]
    pub horizontal: bool,
}

/// Scroll the mouse wheel.
pub struct MouseScrollTool {
    definition: ToolDefinition,
}

impl MouseScrollTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_mouse_scroll",
                "Desktop Mouse Scroll",
                "Scroll the mouse wheel (vertical or horizontal)",
            ),
        }
    }
}

impl Default for MouseScrollTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MouseScrollTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MouseScrollParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let delta = params.delta;

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;
            controller
                .mouse_scroll(params.delta, params.horizontal)
                .map_err(|e| e.to_string())
        })
        .await?;

        debug!("Mouse scrolled by {}", delta);
        Ok(ToolResult::success(format!("Scrolled by {}", delta)))
    }
}
