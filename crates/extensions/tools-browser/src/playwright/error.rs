//! Playwright backend errors.

use thiserror::Error;

/// Playwright backend errors.
#[derive(Debug, Error)]
pub enum PlaywrightError {
    /// Bridge process failed to start.
    #[error("Bridge failed to start: {0}")]
    BridgeStartFailed(String),

    /// Bridge process died unexpectedly.
    #[error("Bridge process died: {0}")]
    BridgeDied(String),

    /// Communication error with bridge.
    #[error("Bridge communication error: {0}")]
    CommunicationError(String),

    /// Bridge returned an error response.
    #[error("Bridge error: {0}")]
    BridgeError(String),

    /// Browser launch failed.
    #[error("Browser launch failed: {0}")]
    BrowserLaunchFailed(String),

    /// Browser connection failed.
    #[error("Browser connection failed: {0}")]
    BrowserConnectFailed(String),

    /// Page not found.
    #[error("Page not found: {0}")]
    PageNotFound(String),

    /// Navigation failed.
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    /// Element not found.
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    /// Action failed.
    #[error("Action failed: {0}")]
    ActionFailed(String),

    /// Screenshot failed.
    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    /// JavaScript execution failed.
    #[error("JavaScript error: {0}")]
    JavaScriptError(String),

    /// DOM processing error.
    #[error("DOM processing error: {0}")]
    DomProcessingError(String),

    /// Timeout waiting for response.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Browser not initialized.
    #[error("Browser not initialized")]
    NotInitialized,

    /// Node.js not found.
    #[error("Node.js not found. Please install Node.js >= 18")]
    NodeNotFound,

    /// Playwright not installed.
    #[error("Playwright not installed. Run: npx playwright install")]
    PlaywrightNotInstalled,
}

impl From<std::io::Error> for PlaywrightError {
    fn from(e: std::io::Error) -> Self {
        PlaywrightError::CommunicationError(e.to_string())
    }
}

impl From<serde_json::Error> for PlaywrightError {
    fn from(e: serde_json::Error) -> Self {
        PlaywrightError::CommunicationError(format!("JSON error: {}", e))
    }
}
