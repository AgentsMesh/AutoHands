use super::*;
use autohands_protocols::Tool;

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
