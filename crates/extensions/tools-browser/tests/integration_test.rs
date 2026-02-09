//! Integration tests for browser tools.
//!
//! These tests require Chrome to be installed on the system.
//! Run with: cargo test -p autohands-tools-browser --test integration_test -- --nocapture

use autohands_tools_browser::manager::{BrowserManager, BrowserManagerConfig};

/// Test helper to create a manager with test-specific config.
fn test_config() -> BrowserManagerConfig {
    BrowserManagerConfig {
        debug_port: 9333, // Use different port to avoid conflicts
        viewport_width: 1280,
        viewport_height: 720,
        profile_dir: Some(std::path::PathBuf::from("/tmp/autohands-test-profile")),
        headless: true, // Use headless for CI
    }
}

#[tokio::test]
async fn test_chrome_detection() {
    // Test that Chrome can be found on the system
    let chrome_path = BrowserManager::find_chrome();
    assert!(chrome_path.is_some(), "Chrome should be installed on the system");

    let path = chrome_path.unwrap();
    println!("Found Chrome at: {}", path.display());
    assert!(path.exists(), "Chrome path should exist");
}

#[tokio::test]
async fn test_connect_and_disconnect() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Connect (should auto-launch Chrome)
    let result = manager.connect().await;
    assert!(result.is_ok(), "Connection should succeed: {:?}", result.err());

    // Close connection
    let result = manager.close().await;
    assert!(result.is_ok(), "Close should succeed");

    // Shutdown Chrome
    let result = manager.shutdown_chrome().await;
    assert!(result.is_ok(), "Shutdown should succeed");
}

#[tokio::test]
async fn test_new_page_and_navigate() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Create a new page
    let page_result = manager.new_page("https://example.com").await;
    assert!(page_result.is_ok(), "New page should succeed: {:?}", page_result.err());

    let page_id = page_result.unwrap();
    println!("Created page: {}", page_id);

    // Wait for page to load
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Get URL
    let url_result = manager.get_url(&page_id).await;
    assert!(url_result.is_ok(), "Get URL should succeed: {:?}", url_result.err());
    let url = url_result.unwrap();
    println!("Page URL: {}", url);
    assert!(url.contains("example.com"), "URL should contain example.com");

    // Get title
    let title_result = manager.get_title(&page_id).await;
    assert!(title_result.is_ok(), "Get title should succeed: {:?}", title_result.err());
    let title = title_result.unwrap();
    println!("Page title: {}", title);
    assert!(!title.is_empty(), "Title should not be empty");

    // Navigate to another page
    let nav_result = manager.navigate(&page_id, "https://httpbin.org/html").await;
    assert!(nav_result.is_ok(), "Navigate should succeed: {:?}", nav_result.err());

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Verify navigation
    let url = manager.get_url(&page_id).await.unwrap();
    println!("New URL: {}", url);
    assert!(url.contains("httpbin.org"), "URL should contain httpbin.org");

    // Close page
    let close_result = manager.close_page(&page_id).await;
    assert!(close_result.is_ok(), "Close page should succeed");

    // Shutdown
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_screenshot() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Create page
    let page_id = manager.new_page("https://example.com").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Take screenshot (viewport only)
    let screenshot_result = manager.screenshot(&page_id, false).await;
    assert!(screenshot_result.is_ok(), "Screenshot should succeed: {:?}", screenshot_result.err());

    let base64_data = screenshot_result.unwrap();
    println!("Screenshot size: {} bytes (base64)", base64_data.len());

    // Verify it's valid base64 and reasonable size
    assert!(!base64_data.is_empty(), "Screenshot should not be empty");
    assert!(base64_data.len() > 1000, "Screenshot should be larger than 1KB");
    assert!(base64_data.len() < 5_000_000, "Screenshot should be less than 5MB (JPEG compressed)");

    // Decode and verify it's a JPEG
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data);
    assert!(decoded.is_ok(), "Should be valid base64");
    let bytes = decoded.unwrap();

    // JPEG magic bytes: FF D8 FF
    assert!(bytes.len() >= 3, "Should have at least 3 bytes");
    assert_eq!(bytes[0], 0xFF, "First byte should be 0xFF (JPEG)");
    assert_eq!(bytes[1], 0xD8, "Second byte should be 0xD8 (JPEG)");
    assert_eq!(bytes[2], 0xFF, "Third byte should be 0xFF (JPEG)");

    println!("Screenshot is valid JPEG, decoded size: {} bytes", bytes.len());

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_page_content() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    let page_id = manager.new_page("https://example.com").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Get page content
    let content_result = manager.get_content(&page_id).await;
    assert!(content_result.is_ok(), "Get content should succeed: {:?}", content_result.err());

    let content = content_result.unwrap();
    println!("Content length: {} chars", content.len());

    // Verify it contains expected HTML
    let content_lower = content.to_lowercase();
    assert!(content_lower.contains("<!doctype") || content_lower.contains("<html"),
            "Should contain DOCTYPE or HTML tag");
    assert!(content.contains("Example Domain"), "Should contain 'Example Domain'");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_javascript_execution() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    let page_id = manager.new_page("https://example.com").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Execute simple JS
    let result = manager.evaluate(&page_id, "1 + 1").await;
    assert!(result.is_ok(), "JS execution should succeed: {:?}", result.err());
    let value = result.unwrap();
    assert_eq!(value, serde_json::json!(2), "1 + 1 should equal 2");

    // Execute DOM query
    let result = manager.evaluate(&page_id, "document.title").await;
    assert!(result.is_ok(), "JS execution should succeed");
    let title = result.unwrap();
    println!("Title from JS: {:?}", title);
    assert!(title.as_str().is_some(), "Title should be a string");

    // Execute complex JS
    let result = manager.evaluate(&page_id, r#"
        (function() {
            return {
                url: window.location.href,
                title: document.title,
                elementCount: document.querySelectorAll('*').length
            };
        })()
    "#).await;
    assert!(result.is_ok(), "Complex JS should succeed");
    let info = result.unwrap();
    println!("Page info: {:?}", info);
    assert!(info.get("url").is_some(), "Should have url");
    assert!(info.get("title").is_some(), "Should have title");
    assert!(info.get("elementCount").is_some(), "Should have elementCount");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_click_and_type() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Use httpbin which has a form
    let page_id = manager.new_page("https://httpbin.org/forms/post").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Fill a text field using selector
    let fill_result = manager.fill(&page_id, "input[name='custname']", "Test User").await;
    assert!(fill_result.is_ok(), "Fill should succeed: {:?}", fill_result.err());

    // Verify the value was entered
    let value = manager.evaluate(&page_id,
        "document.querySelector('input[name=\"custname\"]').value"
    ).await.unwrap();
    println!("Input value: {:?}", value);
    assert_eq!(value.as_str().unwrap(), "Test User", "Input should contain our text");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_scroll() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Use a page with scrollable content
    let page_id = manager.new_page("https://en.wikipedia.org/wiki/Rust_(programming_language)").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Get initial scroll position
    let initial_scroll = manager.evaluate(&page_id, "window.scrollY").await.unwrap();
    println!("Initial scroll: {:?}", initial_scroll);

    // Scroll down
    let scroll_result = manager.scroll(&page_id, 0.0, 500.0).await;
    assert!(scroll_result.is_ok(), "Scroll should succeed: {:?}", scroll_result.err());

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify scroll position changed
    let new_scroll = manager.evaluate(&page_id, "window.scrollY").await.unwrap();
    println!("New scroll: {:?}", new_scroll);

    let initial = initial_scroll.as_f64().unwrap_or(0.0);
    let current = new_scroll.as_f64().unwrap_or(0.0);
    assert!(current > initial, "Scroll position should increase");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_navigation_history() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    let page_id = manager.new_page("https://example.com").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Navigate to second page
    manager.navigate(&page_id, "https://httpbin.org/html").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let url1 = manager.get_url(&page_id).await.unwrap();
    assert!(url1.contains("httpbin"), "Should be on httpbin");

    // Go back
    let back_result = manager.go_back(&page_id).await;
    assert!(back_result.is_ok(), "Go back should succeed");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let url2 = manager.get_url(&page_id).await.unwrap();
    println!("After back: {}", url2);
    assert!(url2.contains("example.com"), "Should be back on example.com");

    // Go forward
    let forward_result = manager.go_forward(&page_id).await;
    assert!(forward_result.is_ok(), "Go forward should succeed");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let url3 = manager.get_url(&page_id).await.unwrap();
    println!("After forward: {}", url3);
    assert!(url3.contains("httpbin"), "Should be back on httpbin");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_multiple_pages() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    // Create multiple pages
    let page1 = manager.new_page("https://example.com").await.unwrap();
    let page2 = manager.new_page("https://httpbin.org/html").await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Verify both pages exist
    let pages = manager.list_pages().await;
    println!("Open pages: {:?}", pages);
    assert_eq!(pages.len(), 2, "Should have 2 pages");
    assert!(pages.contains(&page1), "Should contain page1");
    assert!(pages.contains(&page2), "Should contain page2");

    // Verify they have different URLs
    let url1 = manager.get_url(&page1).await.unwrap();
    let url2 = manager.get_url(&page2).await.unwrap();
    assert!(url1.contains("example.com"), "Page1 should be example.com");
    assert!(url2.contains("httpbin"), "Page2 should be httpbin");

    // Close one page
    manager.close_page(&page1).await.unwrap();
    let pages = manager.list_pages().await;
    assert_eq!(pages.len(), 1, "Should have 1 page after closing");

    // Cleanup
    manager.close_page(&page2).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}

#[tokio::test]
async fn test_wait_for_selector() {
    let config = test_config();
    let manager = BrowserManager::new(config);

    let page_id = manager.new_page("https://example.com").await.unwrap();

    // Wait for h1 element (should exist immediately)
    let wait_result = manager.wait_for_selector(&page_id, "h1", Some(5000)).await;
    assert!(wait_result.is_ok(), "Wait for h1 should succeed: {:?}", wait_result.err());

    // Wait for non-existent element (should timeout)
    let wait_result = manager.wait_for_selector(&page_id, "#nonexistent-element-xyz", Some(1000)).await;
    assert!(wait_result.is_err(), "Wait for nonexistent element should fail");

    // Cleanup
    manager.close_page(&page_id).await.unwrap();
    manager.shutdown_chrome().await.unwrap();
}
