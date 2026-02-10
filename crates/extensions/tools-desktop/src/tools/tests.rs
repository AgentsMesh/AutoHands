use super::*;
use autohands_protocols::Tool;
use crate::input::MouseButton;

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
