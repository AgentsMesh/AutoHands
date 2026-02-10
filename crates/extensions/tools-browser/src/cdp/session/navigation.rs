//! Navigation operations for CDP page session.

use serde_json::json;
use tracing::debug;

use crate::cdp::error::CdpError;

use super::core::PageSession;

impl PageSession {
    /// Navigate to URL.
    pub async fn navigate(&self, url: &str) -> Result<String, CdpError> {
        let result = self
            .call("Page.navigate", Some(json!({"url": url})))
            .await?;

        if let Some(error) = result.get("errorText") {
            return Err(CdpError::NavigationFailed(
                error.as_str().unwrap_or("Unknown error").to_string(),
            ));
        }

        let frame_id = result["frameId"]
            .as_str()
            .unwrap_or("main")
            .to_string();

        self.wait_for_load().await?;

        debug!("Navigated to {}", url);
        Ok(frame_id)
    }

    /// Wait for page load.
    pub async fn wait_for_load(&self) -> Result<(), CdpError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);

        loop {
            let result = self.evaluate("document.readyState").await?;

            if let Some(state) = result.as_str() {
                if state == "complete" || state == "interactive" {
                    return Ok(());
                }
            }

            if start.elapsed() > timeout {
                return Err(CdpError::Timeout("Page load timeout".to_string()));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Reload page.
    pub async fn reload(&self) -> Result<(), CdpError> {
        self.call("Page.reload", None).await?;
        self.wait_for_load().await?;
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self) -> Result<(), CdpError> {
        let history = self.call("Page.getNavigationHistory", None).await?;
        let current_index = history["currentIndex"].as_i64().unwrap_or(0);

        if current_index > 0 {
            let entries = history["entries"].as_array();
            if let Some(entries) = entries {
                if let Some(entry) = entries.get((current_index - 1) as usize) {
                    let entry_id = entry["id"].as_i64().unwrap_or(0);
                    self.call(
                        "Page.navigateToHistoryEntry",
                        Some(json!({"entryId": entry_id})),
                    )
                    .await?;
                    self.wait_for_load().await?;
                }
            }
        }
        Ok(())
    }

    /// Go forward.
    pub async fn go_forward(&self) -> Result<(), CdpError> {
        let history = self.call("Page.getNavigationHistory", None).await?;
        let current_index = history["currentIndex"].as_i64().unwrap_or(0);
        let entries = history["entries"].as_array();

        if let Some(entries) = entries {
            if (current_index as usize) < entries.len() - 1 {
                if let Some(entry) = entries.get((current_index + 1) as usize) {
                    let entry_id = entry["id"].as_i64().unwrap_or(0);
                    self.call(
                        "Page.navigateToHistoryEntry",
                        Some(json!({"entryId": entry_id})),
                    )
                    .await?;
                    self.wait_for_load().await?;
                }
            }
        }
        Ok(())
    }

    /// Get current URL.
    pub async fn get_url(&self) -> Result<String, CdpError> {
        let result = self.evaluate("window.location.href").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Get page title.
    pub async fn get_title(&self) -> Result<String, CdpError> {
        let result = self.evaluate("document.title").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Wait for selector to appear.
    pub async fn wait_for_selector(
        &self,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<i64, CdpError> {
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000) as u64);
        let start = std::time::Instant::now();

        loop {
            if let Some(node_id) = self.query_selector(selector).await? {
                return Ok(node_id);
            }

            if start.elapsed() > timeout {
                return Err(CdpError::Timeout(format!(
                    "Waiting for selector '{}' timed out",
                    selector
                )));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Wait for navigation.
    pub async fn wait_for_navigation(&self, timeout_ms: Option<u32>) -> Result<(), CdpError> {
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000) as u64);
        tokio::time::timeout(timeout, self.wait_for_load())
            .await
            .map_err(|_| CdpError::Timeout("Navigation timeout".to_string()))?
    }
}
