//! Keyboard control tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::input::InputController;

use super::run_blocking;

// ============================================================================
// Keyboard Type Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct KeyboardTypeParams {
    pub text: String,
}

/// Type text using the keyboard.
pub struct KeyboardTypeTool {
    definition: ToolDefinition,
}

impl KeyboardTypeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_keyboard_type",
                "Desktop Keyboard Type",
                "Type text using the keyboard",
            ),
        }
    }
}

impl Default for KeyboardTypeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KeyboardTypeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: KeyboardTypeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let len = params.text.len();

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;
            controller.type_text(&params.text).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Typed text");
        Ok(ToolResult::success(format!("Typed {} characters", len)))
    }
}

// ============================================================================
// Keyboard Key Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct KeyboardKeyParams {
    pub key: String,
}

/// Press a single key.
pub struct KeyboardKeyTool {
    definition: ToolDefinition,
}

impl KeyboardKeyTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_keyboard_key",
                "Desktop Keyboard Key",
                "Press a single key (e.g., 'enter', 'tab', 'escape', 'f1')",
            ),
        }
    }
}

impl Default for KeyboardKeyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KeyboardKeyTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: KeyboardKeyParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let key = params.key.clone();

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;
            controller.key_press(&params.key).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Pressed key: {}", key);
        Ok(ToolResult::success(format!("Pressed key: {}", key)))
    }
}

// ============================================================================
// Keyboard Hotkey Tool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct KeyboardHotkeyParams {
    pub keys: Vec<String>,
}

/// Press a key combination (hotkey).
pub struct KeyboardHotkeyTool {
    definition: ToolDefinition,
}

impl KeyboardHotkeyTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_keyboard_hotkey",
                "Desktop Keyboard Hotkey",
                "Press a key combination (e.g., ['ctrl', 'c'] for copy)",
            ),
        }
    }
}

impl Default for KeyboardHotkeyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KeyboardHotkeyTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: KeyboardHotkeyParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let keys_str = params.keys.join("+");

        run_blocking(move || {
            let mut controller = InputController::new().map_err(|e| e.to_string())?;
            let keys: Vec<&str> = params.keys.iter().map(|s| s.as_str()).collect();
            controller.hotkey(&keys).map_err(|e| e.to_string())
        })
        .await?;

        debug!("Pressed hotkey: {}", keys_str);
        Ok(ToolResult::success(format!("Pressed hotkey: {}", keys_str)))
    }
}
