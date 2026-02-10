use super::*;

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 30000);
}

#[test]
fn test_default_content_type() {
    assert_eq!(default_content_type(), "text");
}

#[test]
fn test_navigate_params() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "url": "https://example.com"
    });
    let params: NavigateParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.url, "https://example.com");
    assert_eq!(params.timeout_ms, 30000);
}

#[test]
fn test_navigate_params_with_timeout() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "url": "https://example.com",
        "timeout_ms": 60000
    });
    let params: NavigateParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.timeout_ms, 60000);
}

#[test]
fn test_click_params() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "#button"
    });
    let params: ClickParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.selector, "#button");
}

#[test]
fn test_type_params() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "input",
        "text": "hello"
    });
    let params: TypeTextParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.selector, "input");
    assert_eq!(params.text, "hello");
    assert!(!params.clear_first);
}

#[test]
fn test_type_params_with_clear() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "input",
        "text": "hello",
        "clear_first": true
    });
    let params: TypeTextParams = serde_json::from_value(json).unwrap();
    assert!(params.clear_first);
}

#[test]
fn test_screenshot_params() {
    let json = serde_json::json!({
        "page_id": "page_1"
    });
    let params: ScreenshotParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert!(!params.full_page);
    assert!(params.selector.is_none());
}

#[test]
fn test_screenshot_params_full() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "full_page": true,
        "selector": "#element"
    });
    let params: ScreenshotParams = serde_json::from_value(json).unwrap();
    assert!(params.full_page);
    assert_eq!(params.selector, Some("#element".to_string()));
}

#[test]
fn test_get_content_params() {
    let json = serde_json::json!({
        "page_id": "page_1"
    });
    let params: GetContentParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert!(params.selector.is_none());
    assert_eq!(params.content_type, "text");
}

#[test]
fn test_get_content_params_html() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "div.content",
        "content_type": "html"
    });
    let params: GetContentParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.selector, Some("div.content".to_string()));
    assert_eq!(params.content_type, "html");
}

#[test]
fn test_execute_js_params() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "script": "return document.title"
    });
    let params: ExecuteJsParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.script, "return document.title");
}

#[test]
fn test_wait_for_params() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "#loading"
    });
    let params: WaitForParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.selector, "#loading");
    assert_eq!(params.timeout_ms, 30000);
}

#[test]
fn test_wait_for_params_with_timeout() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "selector": "#loading",
        "timeout_ms": 5000
    });
    let params: WaitForParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.timeout_ms, 5000);
}

#[test]
fn test_screenshot_result_serialize() {
    let result = ScreenshotResult {
        base64: "iVBORw0KGgo=".to_string(),
        width: 1920,
        height: 1080,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("iVBORw0KGgo="));
    assert!(json.contains("1920"));
    assert!(json.contains("1080"));
}

#[test]
fn test_get_dom_params() {
    let json = serde_json::json!({
        "page_id": "page_1"
    });
    let params: GetDomParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert!(params.compact); // default
}

#[test]
fn test_get_dom_params_full() {
    let json = serde_json::json!({
        "page_id": "page_1",
        "compact": false
    });
    let params: GetDomParams = serde_json::from_value(json).unwrap();
    assert!(!params.compact);
}
