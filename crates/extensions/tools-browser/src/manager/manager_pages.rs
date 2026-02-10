//! BrowserManager page management and interaction methods.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::debug;

use crate::cdp::ScreenshotFormat;
use crate::dom::EnhancedNodeTree;
use super::manager_core::PageState;
use super::{BrowserError, BrowserManager};

impl BrowserManager {
    /// Create a new page and navigate to URL.
    pub async fn new_page(&self, url: &str) -> Result<String, BrowserError> {
        self.ensure_connected().await?;
        let client = self.client().await?;

        let session = client.new_page(Some(url)).await?;

        let page_id = {
            let mut counter = self.page_counter.write().await;
            *counter += 1;
            format!("page_{}", *counter)
        };

        self.pages.write().await.insert(
            page_id.clone(),
            PageState {
                session: Arc::new(session),
                url: url.to_string(),
            },
        );

        debug!("Created page {}: {}", page_id, url);
        Ok(page_id)
    }

    /// Close a page.
    pub async fn close_page(&self, page_id: &str) -> Result<(), BrowserError> {
        let state = self.pages.write().await.remove(page_id);
        if let Some(state) = state {
            let client = self.client().await?;
            client.close_page(state.session.target_id()).await?;
        }
        debug!("Closed page {}", page_id);
        Ok(())
    }

    /// List all open pages.
    pub async fn list_pages(&self) -> Vec<String> {
        self.pages.read().await.keys().cloned().collect()
    }

    /// Navigate to URL.
    pub async fn navigate(&self, page_id: &str, url: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.navigate(url).await?;

        if let Some(state) = self.pages.write().await.get_mut(page_id) {
            state.url = url.to_string();
        }

        debug!("Navigated {} to {}", page_id, url);
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self, page_id: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.go_back().await?;
        Ok(())
    }

    /// Go forward.
    pub async fn go_forward(&self, page_id: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.go_forward().await?;
        Ok(())
    }

    /// Reload page.
    pub async fn reload(&self, page_id: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.reload().await?;
        Ok(())
    }

    /// Get current URL.
    pub async fn get_url(&self, page_id: &str) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.get_url().await?)
    }

    /// Get page title.
    pub async fn get_title(&self, page_id: &str) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.get_title().await?)
    }

    /// Click at coordinates.
    pub async fn click(&self, page_id: &str, x: f64, y: f64) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.click(x, y).await?;
        Ok(())
    }

    /// Click on selector.
    pub async fn click_selector(&self, page_id: &str, selector: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.click_selector(selector).await?;
        Ok(())
    }

    /// Type text.
    pub async fn type_text(&self, page_id: &str, text: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.type_text(text).await?;
        Ok(())
    }

    /// Fill input field.
    pub async fn fill(&self, page_id: &str, selector: &str, value: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.fill(selector, value).await?;
        Ok(())
    }

    /// Press key.
    pub async fn press_key(&self, page_id: &str, key: &str) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.press_key(key).await?;
        Ok(())
    }

    /// Scroll page.
    pub async fn scroll(&self, page_id: &str, x: f64, y: f64) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        let center_x = self.config.viewport_width as f64 / 2.0;
        let center_y = self.config.viewport_height as f64 / 2.0;
        session.scroll(center_x, center_y, x, y).await?;
        Ok(())
    }

    /// Take screenshot (returns base64 JPEG with quality compression).
    pub async fn screenshot(&self, page_id: &str, full_page: bool) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session
            .screenshot(ScreenshotFormat::Jpeg, Some(60), full_page, None)
            .await?)
    }

    /// Take screenshot with custom format and quality.
    pub async fn screenshot_with_options(
        &self, page_id: &str, full_page: bool,
        format: ScreenshotFormat, quality: Option<u8>,
    ) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.screenshot(format, quality, full_page, None).await?)
    }

    /// Get page HTML content.
    pub async fn get_content(&self, page_id: &str) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.get_content().await?)
    }

    /// Execute JavaScript.
    pub async fn evaluate(&self, page_id: &str, script: &str) -> Result<serde_json::Value, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.evaluate(script).await?)
    }

    /// Wait for selector.
    pub async fn wait_for_selector(
        &self, page_id: &str, selector: &str, timeout_ms: Option<u32>,
    ) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.wait_for_selector(selector, timeout_ms).await?;
        Ok(())
    }

    /// Get enhanced DOM tree with clickability analysis.
    pub async fn get_dom_tree(&self, page_id: &str) -> Result<EnhancedNodeTree, BrowserError> {
        let session = self.get_session(page_id).await?;
        let url = session.get_url().await?;
        let title = session.get_title().await?;

        Ok(EnhancedNodeTree {
            roots: vec![],
            nodes: HashMap::new(),
            viewport: crate::dom::ViewportInfo {
                width: self.config.viewport_width,
                height: self.config.viewport_height,
                device_pixel_ratio: 1.0,
                scroll_x: 0.0,
                scroll_y: 0.0,
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            url,
            title,
        })
    }

    /// Get element at coordinates.
    pub async fn element_at(
        &self, page_id: &str, x: f64, y: f64,
    ) -> Result<Option<serde_json::Value>, BrowserError> {
        let session = self.get_session(page_id).await?;

        let script = format!(
            "JSON.stringify((function() {{
                const el = document.elementFromPoint({}, {});
                if (!el) return null;
                return {{
                    tagName: el.tagName.toLowerCase(),
                    id: el.id || null,
                    className: el.className || null,
                    textContent: (el.textContent || '').substring(0, 100)
                }};
            }})())",
            x, y
        );

        let result = session.evaluate(&script).await?;
        if result.is_null() {
            Ok(None)
        } else if let Some(s) = result.as_str() {
            Ok(Some(serde_json::from_str(s).unwrap_or(serde_json::Value::Null)))
        } else {
            Ok(Some(result))
        }
    }

    /// Get page representation for LLM.
    pub async fn get_page_for_llm(&self, page_id: &str) -> Result<String, BrowserError> {
        let dom_tree = self.get_dom_tree(page_id).await?;
        Ok(dom_tree.to_llm_string())
    }
}
