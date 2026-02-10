use super::*;

#[test]
fn test_window_error_display() {
    let err = WindowError::NotFound("test".to_string());
    assert!(err.to_string().contains("test"));
}

#[test]
fn test_window_info_serialize() {
    let info = WindowInfo {
        id: 123,
        title: "Test Window".to_string(),
        app_name: "Test App".to_string(),
        pid: 456,
        x: 100,
        y: 200,
        width: 800,
        height: 600,
        is_minimized: false,
        is_maximized: false,
        is_focused: true,
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("Test Window"));
    assert!(json.contains("123"));
}

#[test]
fn test_window_controller_new() {
    let controller = WindowController::new();
    assert!(controller.is_ok());
}

#[test]
fn test_window_controller_default() {
    let _controller = WindowController::default();
}
