use super::*;

#[test]
fn test_config_default() {
    let config = PlaywrightBridgeConfig::default();
    assert!(config.node_path.is_none());
    assert_eq!(config.response_timeout_ms, 30000);
}

#[test]
fn test_screenshot_options_serialize() {
    let options = ScreenshotOptions {
        full_page: Some(true),
        clip: None,
        quality: Some(80),
        format: Some("jpeg".to_string()),
    };

    let json = serde_json::to_string(&options).unwrap();
    assert!(json.contains("fullPage"));
    assert!(json.contains("true"));
    assert!(json.contains("quality"));
    assert!(json.contains("80"));
}

#[test]
fn test_clip_region_serialize() {
    let clip = ClipRegion {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
    };

    let json = serde_json::to_string(&clip).unwrap();
    assert!(json.contains("10"));
    assert!(json.contains("100"));
}
