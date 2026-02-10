use super::*;

#[test]
fn test_config_default() {
    let config = PlaywrightManagerConfig::default();
    assert!(config.headless);
    assert_eq!(config.viewport_width, 1280);
    assert_eq!(config.viewport_height, 720);
    assert!(config.connect_url.is_none());
}

#[test]
fn test_config_custom() {
    let config = PlaywrightManagerConfig {
        headless: false,
        viewport_width: 1920,
        viewport_height: 1080,
        connect_url: Some("http://localhost:9222".to_string()),
        browser_args: vec!["--disable-gpu".to_string()],
        ..Default::default()
    };
    assert!(!config.headless);
    assert_eq!(config.viewport_width, 1920);
    assert_eq!(
        config.connect_url,
        Some("http://localhost:9222".to_string())
    );
}

#[test]
fn test_manager_not_initialized() {
    let config = PlaywrightManagerConfig::default();
    let manager = PlaywrightManager::new(config);
    assert!(!manager.is_initialized());
    assert!(manager.list_pages().is_empty());
}
