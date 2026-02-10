//! Browser manager type definitions and configuration.

use std::path::PathBuf;

use thiserror::Error;

use crate::cdp::CdpError;

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
