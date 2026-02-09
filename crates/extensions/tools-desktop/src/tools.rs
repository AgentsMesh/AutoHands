//! Desktop automation tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::clipboard::ClipboardController;
use crate::input::{InputController, MouseButton};
use crate::screenshot;

// Helper to run blocking code in a spawned task
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
// Screenshot Tool
// ============================================================================

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

// ============================================================================
// Screen Info Tool
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_params() {
        let json = serde_json::json!({});
        let params: ScreenshotParams = serde_json::from_value(json).unwrap();
        assert!(params.region.is_none());
    }

    #[test]
    fn test_screenshot_params_with_region() {
        let json = serde_json::json!({
            "region": {
                "x": 100,
                "y": 200,
                "width": 300,
                "height": 400
            }
        });
        let params: ScreenshotParams = serde_json::from_value(json).unwrap();
        assert!(params.region.is_some());
        let region = params.region.unwrap();
        assert_eq!(region.x, 100);
        assert_eq!(region.y, 200);
        assert_eq!(region.width, 300);
        assert_eq!(region.height, 400);
    }

    #[test]
    fn test_mouse_move_params() {
        let json = serde_json::json!({
            "x": 100,
            "y": 200
        });
        let params: MouseMoveParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.x, 100);
        assert_eq!(params.y, 200);
        assert!(!params.relative);
    }

    #[test]
    fn test_mouse_move_params_relative() {
        let json = serde_json::json!({
            "x": 10,
            "y": -20,
            "relative": true
        });
        let params: MouseMoveParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.x, 10);
        assert_eq!(params.y, -20);
        assert!(params.relative);
    }

    #[test]
    fn test_mouse_click_params_defaults() {
        let json = serde_json::json!({});
        let params: MouseClickParams = serde_json::from_value(json).unwrap();
        assert!(!params.double_click);
        assert!(matches!(params.button, MouseButton::Left));
        assert!(params.x.is_none());
        assert!(params.y.is_none());
    }

    #[test]
    fn test_mouse_click_params_full() {
        let json = serde_json::json!({
            "button": "right",
            "double_click": true,
            "x": 500,
            "y": 600
        });
        let params: MouseClickParams = serde_json::from_value(json).unwrap();
        assert!(params.double_click);
        assert!(matches!(params.button, MouseButton::Right));
        assert_eq!(params.x, Some(500));
        assert_eq!(params.y, Some(600));
    }

    #[test]
    fn test_mouse_scroll_params() {
        let json = serde_json::json!({
            "delta": 120
        });
        let params: MouseScrollParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.delta, 120);
        assert!(!params.horizontal);
    }

    #[test]
    fn test_mouse_scroll_params_horizontal() {
        let json = serde_json::json!({
            "delta": -60,
            "horizontal": true
        });
        let params: MouseScrollParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.delta, -60);
        assert!(params.horizontal);
    }

    #[test]
    fn test_keyboard_type_params() {
        let json = serde_json::json!({
            "text": "Hello, World!"
        });
        let params: KeyboardTypeParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.text, "Hello, World!");
    }

    #[test]
    fn test_keyboard_key_params() {
        let json = serde_json::json!({
            "key": "enter"
        });
        let params: KeyboardKeyParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.key, "enter");
    }

    #[test]
    fn test_keyboard_hotkey_params() {
        let json = serde_json::json!({
            "keys": ["ctrl", "c"]
        });
        let params: KeyboardHotkeyParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.keys.len(), 2);
        assert_eq!(params.keys[0], "ctrl");
        assert_eq!(params.keys[1], "c");
    }

    #[test]
    fn test_clipboard_get_params_defaults() {
        let json = serde_json::json!({});
        let params: ClipboardGetParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.content_type, "text");
    }

    #[test]
    fn test_clipboard_get_params_image() {
        let json = serde_json::json!({
            "content_type": "image"
        });
        let params: ClipboardGetParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.content_type, "image");
    }

    #[test]
    fn test_clipboard_set_params() {
        let json = serde_json::json!({
            "text": "clipboard content"
        });
        let params: ClipboardSetParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.text, "clipboard content");
    }

    #[test]
    fn test_default_button() {
        assert!(matches!(default_button(), MouseButton::Left));
    }

    #[test]
    fn test_default_content_type() {
        assert_eq!(default_content_type(), "text");
    }

    // Tool definition tests
    #[test]
    fn test_desktop_screenshot_tool_definition() {
        let tool = DesktopScreenshotTool::new();
        assert_eq!(tool.definition().id, "desktop_screenshot");
    }

    #[test]
    fn test_screen_info_tool_definition() {
        let tool = ScreenInfoTool::new();
        assert_eq!(tool.definition().id, "desktop_screen_info");
    }

    #[test]
    fn test_mouse_move_tool_definition() {
        let tool = MouseMoveTool::new();
        assert_eq!(tool.definition().id, "desktop_mouse_move");
    }

    #[test]
    fn test_mouse_click_tool_definition() {
        let tool = MouseClickTool::new();
        assert_eq!(tool.definition().id, "desktop_mouse_click");
    }

    #[test]
    fn test_mouse_scroll_tool_definition() {
        let tool = MouseScrollTool::new();
        assert_eq!(tool.definition().id, "desktop_mouse_scroll");
    }

    #[test]
    fn test_keyboard_type_tool_definition() {
        let tool = KeyboardTypeTool::new();
        assert_eq!(tool.definition().id, "desktop_keyboard_type");
    }

    #[test]
    fn test_keyboard_key_tool_definition() {
        let tool = KeyboardKeyTool::new();
        assert_eq!(tool.definition().id, "desktop_keyboard_key");
    }

    #[test]
    fn test_keyboard_hotkey_tool_definition() {
        let tool = KeyboardHotkeyTool::new();
        assert_eq!(tool.definition().id, "desktop_keyboard_hotkey");
    }

    #[test]
    fn test_clipboard_get_tool_definition() {
        let tool = ClipboardGetTool::new();
        assert_eq!(tool.definition().id, "desktop_clipboard_get");
    }

    #[test]
    fn test_clipboard_set_tool_definition() {
        let tool = ClipboardSetTool::new();
        assert_eq!(tool.definition().id, "desktop_clipboard_set");
    }

    // Default impl tests
    #[test]
    fn test_tools_default_impl() {
        let _ = DesktopScreenshotTool::default();
        let _ = ScreenInfoTool::default();
        let _ = MouseMoveTool::default();
        let _ = MouseClickTool::default();
        let _ = MouseScrollTool::default();
        let _ = KeyboardTypeTool::default();
        let _ = KeyboardKeyTool::default();
        let _ = KeyboardHotkeyTool::default();
        let _ = ClipboardGetTool::default();
        let _ = ClipboardSetTool::default();
    }
}
