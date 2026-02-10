//! BrowserManager core: struct definition, new, connect, chrome management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::cdp::{CdpClient, PageSession};
use super::{BrowserError, BrowserManagerConfig};

/// Page state tracking.
pub(super) struct PageState {
    pub(super) session: Arc<PageSession>,
    pub(super) url: String,
}

/// Manages browser connections and pages.
pub struct BrowserManager {
    pub(super) config: BrowserManagerConfig,
    pub(super) client: RwLock<Option<Arc<CdpClient>>>,
    pub(super) pages: RwLock<HashMap<String, PageState>>,
    pub(super) page_counter: RwLock<u64>,
    /// Chrome process handle (if we launched it).
    pub(super) chrome_process: RwLock<Option<Child>>,
}

impl BrowserManager {
    /// Create a new browser manager.
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
    pub(super) async fn is_chrome_running(&self) -> bool {
        reqwest::get(&format!("{}/json/version", self.config.endpoint()))
            .await
            .is_ok()
    }

    /// Launch Chrome with remote debugging enabled.
    pub(super) async fn launch_chrome(&self) -> Result<Child, BrowserError> {
        let chrome_path = Self::find_chrome().ok_or(BrowserError::ChromeNotFound)?;
        let profile_dir = self.config.get_profile_dir();

        if let Err(e) = std::fs::create_dir_all(&profile_dir) {
            warn!("Failed to create profile directory: {}", e);
        }

        info!("Launching Chrome with profile at: {}", profile_dir.display());

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

        if !self.is_chrome_running().await {
            info!("Chrome not running on port {}, launching...", self.config.debug_port);

            let child = self.launch_chrome().await?;
            *self.chrome_process.write().await = Some(child);

            let mut attempts = 0;
            let max_attempts = 30;
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
    pub(super) async fn client(&self) -> Result<Arc<CdpClient>, BrowserError> {
        self.client
            .read()
            .await
            .clone()
            .ok_or(BrowserError::NotConnected)
    }

    /// Get page session by ID (clones the Arc).
    pub(super) async fn get_session(&self, page_id: &str) -> Result<Arc<PageSession>, BrowserError> {
        let pages = self.pages.read().await;
        let state = pages
            .get(page_id)
            .ok_or_else(|| BrowserError::PageNotFound(page_id.to_string()))?;
        Ok(state.session.clone())
    }

    /// Close the browser connection.
    pub async fn close(&self) -> Result<(), BrowserError> {
        self.pages.write().await.clear();
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
}
