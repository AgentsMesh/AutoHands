//! High-level Playwright browser manager.
//!
//! This module provides a high-level API for browser automation using Playwright.
//! It manages browser instances, pages, and provides simplified methods for
//! common operations.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{debug, info};

use super::bridge::{PlaywrightBridge, PlaywrightBridgeConfig, ScreenshotOptions};
use super::dom::EnhancedNodeTree;
use super::error::PlaywrightError;

/// Playwright manager configuration.
#[derive(Debug, Clone)]
pub struct PlaywrightManagerConfig {
    /// Run in headless mode.
    pub headless: bool,
    /// Default viewport width.
    pub viewport_width: u32,
    /// Default viewport height.
    pub viewport_height: u32,
    /// Connect to existing browser via CDP endpoint.
    /// e.g., "http://localhost:9222"
    pub connect_url: Option<String>,
    /// Additional browser args.
    pub browser_args: Vec<String>,
    /// Bridge configuration.
    pub bridge_config: PlaywrightBridgeConfig,
}

impl Default for PlaywrightManagerConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1280,
            viewport_height: 720,
            connect_url: None,
            browser_args: vec![],
            bridge_config: PlaywrightBridgeConfig::default(),
        }
    }
}

/// Page state tracking.
struct PageState {
    page_id: String,
    browser_id: String,
    url: String,
}

/// High-level Playwright browser manager.
pub struct PlaywrightManager {
    config: PlaywrightManagerConfig,
    bridge: Arc<PlaywrightBridge>,
    browser_id: RwLock<Option<String>>,
    pages: RwLock<HashMap<String, PageState>>,
    page_counter: RwLock<u64>,
    initialized: std::sync::atomic::AtomicBool,
}

impl PlaywrightManager {
    /// Create a new Playwright manager.
    pub fn new(config: PlaywrightManagerConfig) -> Self {
        let bridge = Arc::new(PlaywrightBridge::new(config.bridge_config.clone()));

        Self {
            config,
            bridge,
            browser_id: RwLock::new(None),
            pages: RwLock::new(HashMap::new()),
            page_counter: RwLock::new(0),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Check if the manager is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Initialize the browser.
    pub async fn initialize(&self) -> Result<(), PlaywrightError> {
        if self.is_initialized() {
            return Ok(());
        }

        // Start bridge
        self.bridge.start().await?;

        // Launch or connect browser
        let browser_id = if let Some(ref endpoint) = self.config.connect_url {
            info!("Connecting to existing browser at {}", endpoint);
            self.bridge.connect_browser(endpoint).await?
        } else {
            info!(
                "Launching new browser (headless={})",
                self.config.headless
            );
            self.bridge
                .launch_browser(self.config.headless, self.config.browser_args.clone())
                .await?
        };

        *self.browser_id.write() = Some(browser_id);
        self.initialized
            .store(true, std::sync::atomic::Ordering::SeqCst);

        info!("Playwright manager initialized");
        Ok(())
    }

    /// Ensure manager is initialized (lazy init).
    pub async fn ensure_initialized(&self) -> Result<(), PlaywrightError> {
        if !self.is_initialized() {
            self.initialize().await?;
        }
        Ok(())
    }

    /// Close the browser and manager.
    pub async fn close(&self) -> Result<(), PlaywrightError> {
        // Close all pages - collect page IDs first, then release lock before await
        let page_ids: Vec<String> = self.pages.read().keys().cloned().collect();
        for page_id in page_ids {
            let _ = self.close_page(&page_id).await;
        }

        // Close browser - take browser_id out first, then release lock before await
        let browser_id = self.browser_id.write().take();
        if let Some(browser_id) = browser_id {
            self.bridge.close_browser(&browser_id).await?;
        }

        // Stop bridge
        self.bridge.stop().await?;

        self.initialized
            .store(false, std::sync::atomic::Ordering::SeqCst);

        info!("Playwright manager closed");
        Ok(())
    }

    // ============================================================================
    // Page Management
    // ============================================================================

    /// Create a new page and navigate to URL.
    pub async fn new_page(&self, url: &str) -> Result<String, PlaywrightError> {
        self.ensure_initialized().await?;

        let browser_id = self
            .browser_id
            .read()
            .clone()
            .ok_or(PlaywrightError::NotInitialized)?;

        // Create page via bridge
        let page_id = self.bridge.new_page(&browser_id).await?;

        // Navigate
        self.bridge
            .navigate(&page_id, url, Some("domcontentloaded"))
            .await?;

        // Generate our own page ID
        let our_page_id = {
            let mut counter = self.page_counter.write();
            *counter += 1;
            format!("page_{}", *counter)
        };

        // Store state
        self.pages.write().insert(
            our_page_id.clone(),
            PageState {
                page_id,
                browser_id,
                url: url.to_string(),
            },
        );

        debug!("Created page {}: {}", our_page_id, url);
        Ok(our_page_id)
    }

    /// Get internal page ID for a public page ID.
    fn get_internal_page_id(&self, page_id: &str) -> Result<String, PlaywrightError> {
        self.pages
            .read()
            .get(page_id)
            .map(|s| s.page_id.clone())
            .ok_or_else(|| PlaywrightError::PageNotFound(page_id.to_string()))
    }

    /// Close a page.
    pub async fn close_page(&self, page_id: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.close_page(&internal_id).await?;
        self.pages.write().remove(page_id);
        debug!("Closed page {}", page_id);
        Ok(())
    }

    /// List all open pages.
    pub fn list_pages(&self) -> Vec<String> {
        self.pages.read().keys().cloned().collect()
    }

    // ============================================================================
    // Navigation
    // ============================================================================

    /// Navigate to URL.
    pub async fn navigate(&self, page_id: &str, url: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge
            .navigate(&internal_id, url, Some("domcontentloaded"))
            .await?;

        // Update stored URL
        if let Some(state) = self.pages.write().get_mut(page_id) {
            state.url = url.to_string();
        }

        debug!("Navigated {} to {}", page_id, url);
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self, page_id: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.go_back(&internal_id).await
    }

    /// Go forward.
    pub async fn go_forward(&self, page_id: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.go_forward(&internal_id).await
    }

    /// Reload page.
    pub async fn reload(&self, page_id: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.reload(&internal_id).await
    }

    /// Get current URL.
    pub async fn get_url(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.get_url(&internal_id).await
    }

    /// Get page title.
    pub async fn get_title(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.get_title(&internal_id).await
    }

    // ============================================================================
    // Interactions
    // ============================================================================

    /// Click at coordinates.
    pub async fn click(&self, page_id: &str, x: f64, y: f64) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.click(&internal_id, x, y).await?;
        debug!("Clicked at ({}, {}) on {}", x, y, page_id);
        Ok(())
    }

    /// Click on selector.
    pub async fn click_selector(
        &self,
        page_id: &str,
        selector: &str,
    ) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.click_selector(&internal_id, selector).await?;
        debug!("Clicked {} on {}", selector, page_id);
        Ok(())
    }

    /// Type text (keyboard input).
    pub async fn type_text(&self, page_id: &str, text: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.type_text(&internal_id, text, None).await?;
        debug!("Typed {} chars on {}", text.len(), page_id);
        Ok(())
    }

    /// Fill input field by selector.
    pub async fn fill(
        &self,
        page_id: &str,
        selector: &str,
        value: &str,
    ) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.fill(&internal_id, selector, value).await?;
        debug!("Filled {} with {} chars on {}", selector, value.len(), page_id);
        Ok(())
    }

    /// Press key.
    pub async fn press_key(&self, page_id: &str, key: &str) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.press_key(&internal_id, key).await?;
        debug!("Pressed {} on {}", key, page_id);
        Ok(())
    }

    /// Scroll page.
    pub async fn scroll(&self, page_id: &str, x: f64, y: f64) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.scroll(&internal_id, x, y).await?;
        debug!("Scrolled ({}, {}) on {}", x, y, page_id);
        Ok(())
    }

    // ============================================================================
    // Content
    // ============================================================================

    /// Take screenshot (returns base64).
    pub async fn screenshot(
        &self,
        page_id: &str,
        full_page: bool,
    ) -> Result<String, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;

        let options = if full_page {
            Some(ScreenshotOptions {
                full_page: Some(true),
                clip: None,
                quality: None,
                format: Some("png".to_string()),
            })
        } else {
            None
        };

        self.bridge.screenshot(&internal_id, options).await
    }

    /// Get page HTML content.
    pub async fn get_content(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.get_content(&internal_id).await
    }

    /// Execute JavaScript.
    pub async fn evaluate(
        &self,
        page_id: &str,
        script: &str,
    ) -> Result<serde_json::Value, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.evaluate(&internal_id, script).await
    }

    /// Wait for selector.
    pub async fn wait_for_selector(
        &self,
        page_id: &str,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<(), PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge
            .wait_for_selector(&internal_id, selector, timeout_ms)
            .await
    }

    // ============================================================================
    // DOM Analysis (Browser-Use Style)
    // ============================================================================

    /// Get enhanced DOM tree with clickability analysis.
    pub async fn get_dom_tree(&self, page_id: &str) -> Result<EnhancedNodeTree, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.get_dom_tree(&internal_id).await
    }

    /// Get element at coordinates.
    pub async fn element_at(
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<Option<serde_json::Value>, PlaywrightError> {
        let internal_id = self.get_internal_page_id(page_id)?;
        self.bridge.element_at(&internal_id, x, y).await
    }

    /// Get page representation for LLM.
    /// Returns a compact, readable description of the page's interactive elements.
    pub async fn get_page_for_llm(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let dom_tree = self.get_dom_tree(page_id).await?;
        Ok(dom_tree.to_llm_string())
    }
}

impl Drop for PlaywrightManager {
    fn drop(&mut self) {
        // Note: async cleanup handled by close() method
    }
}

#[cfg(test)]
mod tests {
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
}
