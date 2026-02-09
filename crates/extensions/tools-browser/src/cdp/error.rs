//! CDP error types.

use thiserror::Error;

/// CDP client errors.
#[derive(Debug, Error)]
pub enum CdpError {
    /// Failed to connect to Chrome.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Chrome not found or not running with remote debugging.
    #[error("Chrome not available at {0}. Start Chrome with: chrome --remote-debugging-port=9222")]
    ChromeNotAvailable(String),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// CDP protocol error.
    #[error("CDP error: {message} (code: {code})")]
    Protocol { code: i64, message: String },

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error (for endpoint discovery).
    #[error("HTTP error: {0}")]
    Http(String),

    /// Page not found.
    #[error("Page not found: {0}")]
    PageNotFound(String),

    /// Navigation failed.
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    /// Element not found.
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    /// JavaScript execution error.
    #[error("JavaScript error: {0}")]
    JavaScript(String),

    /// Timeout.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Session closed.
    #[error("Session closed")]
    SessionClosed,

    /// Invalid response.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for CdpError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        CdpError::WebSocket(e.to_string())
    }
}

impl From<reqwest::Error> for CdpError {
    fn from(e: reqwest::Error) -> Self {
        CdpError::Http(e.to_string())
    }
}

impl From<url::ParseError> for CdpError {
    fn from(e: url::ParseError) -> Self {
        CdpError::ConnectionFailed(format!("Invalid URL: {}", e))
    }
}
