//! Browser API methods for PlaywrightBridge.
//!
//! This module contains browser, page, and DOM interaction methods
//! implemented as a cross-file impl block for PlaywrightBridge.

use super::bridge::{PlaywrightBridge, ScreenshotOptions};
use super::dom::EnhancedNodeTree;
use super::error::PlaywrightError;

impl PlaywrightBridge {
    // ============================================================================
    // Browser Methods
    // ============================================================================

    /// Launch a new browser.
    pub async fn launch_browser(
        &self,
        headless: bool,
        args: Vec<String>,
    ) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "launchBrowser",
                serde_json::json!({
                    "headless": headless,
                    "args": args
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::BrowserLaunchFailed("Invalid response".to_string()))
    }

    /// Connect to an existing browser via CDP.
    pub async fn connect_browser(&self, endpoint: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "connectBrowser",
                serde_json::json!({
                    "endpoint": endpoint
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::BrowserConnectFailed("Invalid response".to_string()))
    }

    /// Close browser.
    pub async fn close_browser(&self, browser_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "closeBrowser",
            serde_json::json!({
                "browserId": browser_id
            }),
        )
        .await?;
        Ok(())
    }

    // ============================================================================
    // Page Methods
    // ============================================================================

    /// Create a new page.
    pub async fn new_page(&self, browser_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "newPage",
                serde_json::json!({
                    "browserId": browser_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid page ID response".to_string()))
    }

    /// Navigate to URL.
    pub async fn navigate(
        &self,
        page_id: &str,
        url: &str,
        wait_until: Option<&str>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "navigate",
            serde_json::json!({
                "pageId": page_id,
                "url": url,
                "waitUntil": wait_until.unwrap_or("domcontentloaded")
            }),
        )
        .await?;
        Ok(())
    }

    /// Click at coordinates.
    pub async fn click(&self, page_id: &str, x: f64, y: f64) -> Result<(), PlaywrightError> {
        self.call(
            "click",
            serde_json::json!({
                "pageId": page_id,
                "x": x,
                "y": y
            }),
        )
        .await?;
        Ok(())
    }

    /// Click on selector.
    pub async fn click_selector(
        &self,
        page_id: &str,
        selector: &str,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "clickSelector",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector
            }),
        )
        .await?;
        Ok(())
    }

    /// Type text.
    pub async fn type_text(
        &self,
        page_id: &str,
        text: &str,
        delay_ms: Option<u32>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "typeText",
            serde_json::json!({
                "pageId": page_id,
                "text": text,
                "delay": delay_ms.unwrap_or(0)
            }),
        )
        .await?;
        Ok(())
    }

    /// Fill input at selector.
    pub async fn fill(
        &self,
        page_id: &str,
        selector: &str,
        value: &str,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "fill",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector,
                "value": value
            }),
        )
        .await?;
        Ok(())
    }

    /// Press key.
    pub async fn press_key(&self, page_id: &str, key: &str) -> Result<(), PlaywrightError> {
        self.call(
            "pressKey",
            serde_json::json!({
                "pageId": page_id,
                "key": key
            }),
        )
        .await?;
        Ok(())
    }

    /// Take screenshot.
    pub async fn screenshot(
        &self,
        page_id: &str,
        options: Option<ScreenshotOptions>,
    ) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "screenshot",
                serde_json::json!({
                    "pageId": page_id,
                    "options": options
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ScreenshotFailed("Invalid response".to_string()))
    }

    /// Get page content.
    pub async fn get_content(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getContent",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid content response".to_string()))
    }

    /// Get page URL.
    pub async fn get_url(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getUrl",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid URL response".to_string()))
    }

    /// Get page title.
    pub async fn get_title(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getTitle",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid title response".to_string()))
    }

    /// Execute JavaScript.
    pub async fn evaluate(
        &self,
        page_id: &str,
        script: &str,
    ) -> Result<serde_json::Value, PlaywrightError> {
        self.call(
            "evaluate",
            serde_json::json!({
                "pageId": page_id,
                "script": script
            }),
        )
        .await
    }

    /// Wait for selector.
    pub async fn wait_for_selector(
        &self,
        page_id: &str,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "waitForSelector",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector,
                "timeout": timeout_ms.unwrap_or(30000)
            }),
        )
        .await?;
        Ok(())
    }

    /// Close page.
    pub async fn close_page(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "closePage",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "goBack",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Go forward.
    pub async fn go_forward(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "goForward",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Reload page.
    pub async fn reload(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "reload",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Scroll page.
    pub async fn scroll(
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "scroll",
            serde_json::json!({
                "pageId": page_id,
                "x": x,
                "y": y
            }),
        )
        .await?;
        Ok(())
    }

    // ============================================================================
    // DOM Analysis Methods (Browser-Use Style)
    // ============================================================================

    /// Get enhanced DOM tree with clickability analysis.
    pub async fn get_dom_tree(&self, page_id: &str) -> Result<EnhancedNodeTree, PlaywrightError> {
        let result = self
            .call(
                "getDomTree",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        serde_json::from_value(result)
            .map_err(|e| PlaywrightError::DomProcessingError(format!("Failed to parse DOM tree: {}", e)))
    }

    /// Get element at coordinates.
    pub async fn element_at(
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<Option<serde_json::Value>, PlaywrightError> {
        let result = self
            .call(
                "elementAt",
                serde_json::json!({
                    "pageId": page_id,
                    "x": x,
                    "y": y
                }),
            )
            .await?;

        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}
