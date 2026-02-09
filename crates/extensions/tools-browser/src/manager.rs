//! Browser instance manager.
//!
//! This module provides a unified browser manager interface using CDP.
//! It automatically launches Chrome with a persistent profile for login state preservation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::cdp::{CdpClient, CdpError, PageSession, ScreenshotFormat};
use crate::dom::EnhancedNodeTree;

/// Browser manager errors.
#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Page not found: {0}")]
    PageNotFound(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Action failed: {0}")]
    ActionFailed(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    #[error("Browser not connected")]
    NotConnected,

    #[error("Chrome not found. Please install Google Chrome.")]
    ChromeNotFound,

    #[error("Failed to launch Chrome: {0}")]
    LaunchFailed(String),
}

impl From<CdpError> for BrowserError {
    fn from(e: CdpError) -> Self {
        match e {
            CdpError::ConnectionFailed(msg) => BrowserError::ConnectionFailed(msg),
            CdpError::ChromeNotAvailable(msg) => BrowserError::ConnectionFailed(msg),
            CdpError::PageNotFound(id) => BrowserError::PageNotFound(id),
            CdpError::NavigationFailed(msg) => BrowserError::NavigationFailed(msg),
            CdpError::ElementNotFound(msg) => BrowserError::ElementNotFound(msg),
            CdpError::JavaScript(msg) => BrowserError::ActionFailed(format!("JS error: {}", msg)),
            CdpError::Timeout(msg) => BrowserError::ActionFailed(format!("Timeout: {}", msg)),
            CdpError::SessionClosed => BrowserError::NotConnected,
            _ => BrowserError::ActionFailed(e.to_string()),
        }
    }
}

/// Browser configuration.
#[derive(Debug, Clone)]
pub struct BrowserManagerConfig {
    /// Chrome debugging port.
    pub debug_port: u16,
    /// Default viewport width.
    pub viewport_width: u32,
    /// Default viewport height.
    pub viewport_height: u32,
    /// Profile directory for persistent login state.
    /// Default: ~/.autohands/browser-profile
    pub profile_dir: Option<PathBuf>,
    /// Whether to run Chrome in headless mode.
    pub headless: bool,
}

impl Default for BrowserManagerConfig {
    fn default() -> Self {
        Self {
            debug_port: 9222,
            viewport_width: 1280,
            viewport_height: 720,
            profile_dir: None,
            headless: false,
        }
    }
}

impl BrowserManagerConfig {
    /// Get the profile directory, creating default if not specified.
    pub fn get_profile_dir(&self) -> PathBuf {
        self.profile_dir.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".autohands")
                .join("browser-profile")
        })
    }

    /// Get the CDP endpoint URL.
    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.debug_port)
    }
}

/// Page state tracking.
struct PageState {
    session: Arc<PageSession>,
    url: String,
}

/// Manages browser connections and pages.
///
/// Key features:
/// - Automatically launches Chrome if not running
/// - Uses persistent profile for login state preservation
/// - Lazily connects on first use
pub struct BrowserManager {
    config: BrowserManagerConfig,
    client: RwLock<Option<Arc<CdpClient>>>,
    pages: RwLock<HashMap<String, PageState>>,
    page_counter: RwLock<u64>,
    /// Chrome process handle (if we launched it).
    chrome_process: RwLock<Option<Child>>,
}

impl BrowserManager {
    /// Create a new browser manager.
    ///
    /// Note: The browser is NOT connected here. It will be lazily connected
    /// on first use (when a tool requires the browser).
    pub fn new(config: BrowserManagerConfig) -> Self {
        Self {
            config,
            client: RwLock::new(None),
            pages: RwLock::new(HashMap::new()),
            page_counter: RwLock::new(0),
            chrome_process: RwLock::new(None),
        }
    }

    /// Find Chrome executable path.
    pub fn find_chrome() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let paths = [
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            ];
            for path in &paths {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let paths = [
                "/usr/bin/google-chrome",
                "/usr/bin/google-chrome-stable",
                "/usr/bin/chromium",
                "/usr/bin/chromium-browser",
                "/snap/bin/chromium",
            ];
            for path in &paths {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let paths = [
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            ];
            for path in &paths {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        None
    }

    /// Check if Chrome is already running on the debug port.
    async fn is_chrome_running(&self) -> bool {
        reqwest::get(&format!("{}/json/version", self.config.endpoint()))
            .await
            .is_ok()
    }

    /// Launch Chrome with remote debugging enabled.
    async fn launch_chrome(&self) -> Result<Child, BrowserError> {
        let chrome_path = Self::find_chrome().ok_or(BrowserError::ChromeNotFound)?;
        let profile_dir = self.config.get_profile_dir();

        // Ensure profile directory exists
        if let Err(e) = std::fs::create_dir_all(&profile_dir) {
            warn!("Failed to create profile directory: {}", e);
        }

        info!(
            "Launching Chrome with profile at: {}",
            profile_dir.display()
        );

        let mut cmd = Command::new(&chrome_path);
        cmd.arg(format!("--remote-debugging-port={}", self.config.debug_port))
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--disable-translate")
            .arg("--metrics-recording-only")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if self.config.headless {
            cmd.arg("--headless=new");
        }

        let child = cmd
            .spawn()
            .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

        info!("Chrome launched with PID: {:?}", child.id());
        Ok(child)
    }

    /// Connect to the browser, launching it if necessary.
    pub async fn connect(&self) -> Result<(), BrowserError> {
        if self.client.read().await.is_some() {
            return Ok(());
        }

        // Check if Chrome is already running
        if !self.is_chrome_running().await {
            info!("Chrome not running on port {}, launching...", self.config.debug_port);

            let child = self.launch_chrome().await?;
            *self.chrome_process.write().await = Some(child);

            // Wait for Chrome to start accepting connections
            let mut attempts = 0;
            let max_attempts = 30; // 30 * 200ms = 6 seconds
            while attempts < max_attempts {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                if self.is_chrome_running().await {
                    break;
                }
                attempts += 1;
            }

            if attempts >= max_attempts {
                return Err(BrowserError::LaunchFailed(
                    "Chrome failed to start within timeout".to_string(),
                ));
            }
        } else {
            info!("Chrome already running on port {}", self.config.debug_port);
        }

        // Connect to Chrome
        let client = CdpClient::connect(&self.config.endpoint()).await?;
        *self.client.write().await = Some(Arc::new(client));

        info!("Connected to Chrome at {}", self.config.endpoint());
        Ok(())
    }

    /// Ensure the browser is connected before use.
    pub async fn ensure_connected(&self) -> Result<(), BrowserError> {
        if self.client.read().await.is_none() {
            self.connect().await?;
        }
        Ok(())
    }

    /// Get the CDP client.
    async fn client(&self) -> Result<Arc<CdpClient>, BrowserError> {
        self.client
            .read()
            .await
            .clone()
            .ok_or(BrowserError::NotConnected)
    }

    /// Close the browser connection.
    /// Note: This does NOT close Chrome itself, only disconnects.
    pub async fn close(&self) -> Result<(), BrowserError> {
        // Clear all pages
        self.pages.write().await.clear();

        // Drop client
        let _ = self.client.write().await.take();

        info!("Browser connection closed");
        Ok(())
    }

    /// Shutdown Chrome if we launched it.
    pub async fn shutdown_chrome(&self) -> Result<(), BrowserError> {
        self.close().await?;

        if let Some(mut child) = self.chrome_process.write().await.take() {
            info!("Shutting down Chrome...");
            let _ = child.kill().await;
        }

        Ok(())
    }

    /// Create a new page and navigate to URL.
    pub async fn new_page(&self, url: &str) -> Result<String, BrowserError> {
        self.ensure_connected().await?;
        let client = self.client().await?;

        // Create new page via CDP
        let session = client.new_page(Some(url)).await?;

        // Generate our own page ID
        let page_id = {
            let mut counter = self.page_counter.write().await;
            *counter += 1;
            format!("page_{}", *counter)
        };

        // Store state
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

    /// Get page session by ID (clones the Arc).
    async fn get_session(&self, page_id: &str) -> Result<Arc<PageSession>, BrowserError> {
        let pages = self.pages.read().await;
        let state = pages
            .get(page_id)
            .ok_or_else(|| BrowserError::PageNotFound(page_id.to_string()))?;
        Ok(state.session.clone())
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

        // Update stored URL
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
    pub async fn click_selector(
        &self,
        page_id: &str,
        selector: &str,
    ) -> Result<(), BrowserError> {
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
    pub async fn fill(
        &self,
        page_id: &str,
        selector: &str,
        value: &str,
    ) -> Result<(), BrowserError> {
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
        // Scroll at center of viewport
        let center_x = self.config.viewport_width as f64 / 2.0;
        let center_y = self.config.viewport_height as f64 / 2.0;
        session.scroll(center_x, center_y, x, y).await?;
        Ok(())
    }

    /// Take screenshot (returns base64 JPEG with quality compression).
    ///
    /// Uses JPEG format with 60% quality by default to reduce size for LLM consumption.
    pub async fn screenshot(
        &self,
        page_id: &str,
        full_page: bool,
    ) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        // Use JPEG with 60% quality to reduce size (PNG can be 5-10x larger)
        Ok(session
            .screenshot(ScreenshotFormat::Jpeg, Some(60), full_page, None)
            .await?)
    }

    /// Take screenshot with custom format and quality.
    pub async fn screenshot_with_options(
        &self,
        page_id: &str,
        full_page: bool,
        format: ScreenshotFormat,
        quality: Option<u8>,
    ) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session
            .screenshot(format, quality, full_page, None)
            .await?)
    }

    /// Get page HTML content.
    pub async fn get_content(&self, page_id: &str) -> Result<String, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.get_content().await?)
    }

    /// Execute JavaScript.
    pub async fn evaluate(
        &self,
        page_id: &str,
        script: &str,
    ) -> Result<serde_json::Value, BrowserError> {
        let session = self.get_session(page_id).await?;
        Ok(session.evaluate(script).await?)
    }

    /// Wait for selector.
    pub async fn wait_for_selector(
        &self,
        page_id: &str,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<(), BrowserError> {
        let session = self.get_session(page_id).await?;
        session.wait_for_selector(selector, timeout_ms).await?;
        Ok(())
    }

    // ============================================================================
    // DOM Analysis (Browser-Use Style)
    // ============================================================================

    /// Get enhanced DOM tree with clickability analysis.
    ///
    /// TODO: Implement full DOM tree building with clickability scores.
    /// For now, returns a placeholder.
    pub async fn get_dom_tree(&self, page_id: &str) -> Result<EnhancedNodeTree, BrowserError> {
        let session = self.get_session(page_id).await?;

        // Get basic page info
        let url = session.get_url().await?;
        let title = session.get_title().await?;

        // TODO: Implement full DOM tree building
        // For now, return a minimal tree
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
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<Option<serde_json::Value>, BrowserError> {
        let session = self.get_session(page_id).await?;

        // Use JavaScript to find element at point
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

#[cfg(test)]
mod tests {
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
        // This may or may not find Chrome depending on the system
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
}
