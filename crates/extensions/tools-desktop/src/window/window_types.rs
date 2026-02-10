//! Window management type definitions.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors related to window operations.
#[derive(Debug, Error)]
pub enum WindowError {
    /// Platform not supported.
    #[error("Window management not supported on this platform")]
    PlatformNotSupported,

    /// Failed to list windows.
    #[error("Failed to list windows: {0}")]
    ListFailed(String),

    /// Failed to focus window.
    #[error("Failed to focus window: {0}")]
    FocusFailed(String),

    /// Failed to move window.
    #[error("Failed to move window: {0}")]
    MoveFailed(String),

    /// Failed to resize window.
    #[error("Failed to resize window: {0}")]
    ResizeFailed(String),

    /// Window not found.
    #[error("Window not found: {0}")]
    NotFound(String),
}

/// Information about a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Window ID (platform-specific).
    pub id: u64,
    /// Window title.
    pub title: String,
    /// Application name.
    pub app_name: String,
    /// Process ID.
    pub pid: u32,
    /// Window position X.
    pub x: i32,
    /// Window position Y.
    pub y: i32,
    /// Window width.
    pub width: u32,
    /// Window height.
    pub height: u32,
    /// Whether the window is minimized.
    pub is_minimized: bool,
    /// Whether the window is maximized.
    pub is_maximized: bool,
    /// Whether the window has focus.
    pub is_focused: bool,
}
