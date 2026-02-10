use super::*;

#[test]
fn test_config_default() {
    let config = BrowserManagerConfig::default();
    assert_eq!(config.debug_port, 9222);
    assert_eq!(config.viewport_width, 1280);
    assert_eq!(config.viewport_height, 720);
    assert!(!config.headless);
}

#[test]
fn test_config_endpoint() {
    let config = BrowserManagerConfig::default();
    assert_eq!(config.endpoint(), "http://localhost:9222");
}

#[test]
fn test_config_profile_dir() {
    let config = BrowserManagerConfig::default();
    let profile = config.get_profile_dir();
    assert!(profile.ends_with(".autohands/browser-profile"));
}

#[test]
fn test_browser_error_display() {
    let err = BrowserError::ConnectionFailed("timeout".to_string());
    assert_eq!(err.to_string(), "Connection failed: timeout");

    let err = BrowserError::ChromeNotFound;
    assert_eq!(err.to_string(), "Chrome not found. Please install Google Chrome.");

    let err = BrowserError::LaunchFailed("permission denied".to_string());
    assert_eq!(err.to_string(), "Failed to launch Chrome: permission denied");
}

#[test]
fn test_find_chrome() {
    let _result = BrowserManager::find_chrome();
}

#[tokio::test]
async fn test_close_without_connect() {
    let manager = BrowserManager::new(BrowserManagerConfig::default());
    let result = manager.close().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_pages_empty() {
    let manager = BrowserManager::new(BrowserManagerConfig::default());
    assert!(manager.list_pages().await.is_empty());
}
