use super::*;

#[test]
fn test_cdp_request_serialize() {
    let req = CdpRequest {
        id: 1,
        method: "Page.navigate".to_string(),
        params: Some(serde_json::json!({"url": "https://example.com"})),
        session_id: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("Page.navigate"));
    assert!(json.contains("example.com"));
}

#[test]
fn test_cdp_response_deserialize() {
    let json = r#"{"id": 1, "result": {"frameId": "abc"}}"#;
    let resp: CdpResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.id, Some(1));
    assert!(resp.result.is_some());
}

#[test]
fn test_page_info_deserialize() {
    let json = r#"{
        "id": "page123",
        "type": "page",
        "title": "Test",
        "url": "https://example.com",
        "webSocketDebuggerUrl": "ws://localhost:9222/devtools/page/page123"
    }"#;
    let info: PageInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.id, "page123");
    assert_eq!(info.page_type, "page");
}

#[test]
fn test_mouse_button_serialize() {
    let btn = MouseButton::Left;
    let json = serde_json::to_string(&btn).unwrap();
    assert_eq!(json, "\"left\"");
}

#[test]
fn test_screenshot_format_serialize() {
    let fmt = ScreenshotFormat::Png;
    let json = serde_json::to_string(&fmt).unwrap();
    assert_eq!(json, "\"png\"");
}
