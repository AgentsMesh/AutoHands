use super::*;

#[test]
fn test_default_html_content() {
    let html = default_index_html();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("AutoHands"));
    assert!(html.contains("messages"));
}

#[test]
fn test_default_css_content() {
    let css = default_style_css();
    assert!(css.contains("body"));
    assert!(css.contains(".message"));
}

#[test]
fn test_default_js_content() {
    let js = default_app_js();
    assert!(js.contains("WebSocket"));
    assert!(js.contains("connect"));
}

#[test]
fn test_create_router() {
    let state = Arc::new(WebChannelState::new("web"));
    let _router = create_router(state);
    // Router should be created without panicking
}
