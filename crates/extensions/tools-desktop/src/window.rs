//! Window management functionality.
//!
//! Provides cross-platform window control capabilities.

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

/// Window controller for managing system windows.
pub struct WindowController;

impl WindowController {
    /// Create a new window controller.
    pub fn new() -> Result<Self, WindowError> {
        Ok(Self)
    }

    /// List all windows.
    #[cfg(target_os = "macos")]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
        use std::process::Command;

        // Use AppleScript to list windows
        let script = r#"
            tell application "System Events"
                set windowList to {}
                repeat with proc in (processes whose visible is true)
                    set procName to name of proc
                    set procId to unix id of proc
                    try
                        repeat with win in windows of proc
                            set winTitle to name of win
                            set winPos to position of win
                            set winSize to size of win
                            set end of windowList to {procName, procId, winTitle, item 1 of winPos, item 2 of winPos, item 1 of winSize, item 2 of winSize}
                        end repeat
                    end try
                end repeat
                return windowList
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| WindowError::ListFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::ListFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_applescript_windows(&stdout)
    }

    /// List all windows.
    #[cfg(target_os = "linux")]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
        use std::process::Command;

        // Use wmctrl to list windows
        let output = Command::new("wmctrl")
            .arg("-l")
            .arg("-p")
            .arg("-G")
            .output()
            .map_err(|e| WindowError::ListFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::ListFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_wmctrl_windows(&stdout)
    }

    /// List all windows.
    #[cfg(target_os = "windows")]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
        // Windows implementation using Win32 API would go here
        // For now, return empty list
        Ok(Vec::new())
    }

    /// List all windows (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Focus a window by ID.
    #[cfg(target_os = "macos")]
    pub fn focus_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        // AppleScript to focus window by index (simplified)
        let script = format!(
            r#"
            tell application "System Events"
                set frontmost of process id {} to true
            end tell
            "#,
            id
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::FocusFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::FocusFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Focus a window by ID.
    #[cfg(target_os = "linux")]
    pub fn focus_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("wmctrl")
            .arg("-i")
            .arg("-a")
            .arg(format!("0x{:x}", id))
            .output()
            .map_err(|e| WindowError::FocusFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::FocusFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Focus a window by ID (Windows).
    #[cfg(target_os = "windows")]
    pub fn focus_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Focus a window by ID (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn focus_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Move a window.
    #[cfg(target_os = "macos")]
    pub fn move_window(&self, id: u64, x: i32, y: i32) -> Result<(), WindowError> {
        use std::process::Command;

        let script = format!(
            r#"
            tell application "System Events"
                try
                    set position of window 1 of process id {} to {{{}, {}}}
                end try
            end tell
            "#,
            id, x, y
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Move a window.
    #[cfg(target_os = "linux")]
    pub fn move_window(&self, id: u64, x: i32, y: i32) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("wmctrl")
            .arg("-i")
            .arg("-r")
            .arg(format!("0x{:x}", id))
            .arg("-e")
            .arg(format!("0,{},{}", x, y))
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Move a window (Windows).
    #[cfg(target_os = "windows")]
    pub fn move_window(&self, _id: u64, _x: i32, _y: i32) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Move a window (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn move_window(&self, _id: u64, _x: i32, _y: i32) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Resize a window.
    #[cfg(target_os = "macos")]
    pub fn resize_window(&self, id: u64, width: u32, height: u32) -> Result<(), WindowError> {
        use std::process::Command;

        let script = format!(
            r#"
            tell application "System Events"
                try
                    set size of window 1 of process id {} to {{{}, {}}}
                end try
            end tell
            "#,
            id, width, height
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::ResizeFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::ResizeFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Resize a window.
    #[cfg(target_os = "linux")]
    pub fn resize_window(&self, id: u64, width: u32, height: u32) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("wmctrl")
            .arg("-i")
            .arg("-r")
            .arg(format!("0x{:x}", id))
            .arg("-e")
            .arg(format!("0,-1,-1,{},{}", width, height))
            .output()
            .map_err(|e| WindowError::ResizeFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::ResizeFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Resize a window (Windows).
    #[cfg(target_os = "windows")]
    pub fn resize_window(&self, _id: u64, _width: u32, _height: u32) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Resize a window (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn resize_window(&self, _id: u64, _width: u32, _height: u32) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Minimize a window.
    #[cfg(target_os = "macos")]
    pub fn minimize_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let script = format!(
            r#"
            tell application "System Events"
                try
                    set miniaturized of window 1 of process id {} to true
                end try
            end tell
            "#,
            id
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Minimize a window.
    #[cfg(target_os = "linux")]
    pub fn minimize_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("xdotool")
            .arg("windowminimize")
            .arg(format!("{}", id))
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Minimize a window (Windows).
    #[cfg(target_os = "windows")]
    pub fn minimize_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Minimize a window (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn minimize_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Maximize a window.
    #[cfg(target_os = "macos")]
    pub fn maximize_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        // macOS uses "zoom" for maximize
        let script = format!(
            r#"
            tell application "System Events"
                try
                    click (first button of window 1 of process id {} whose subrole is "AXFullScreenButton")
                on error
                    keystroke "f" using {{control down, command down}}
                end try
            end tell
            "#,
            id
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Maximize a window.
    #[cfg(target_os = "linux")]
    pub fn maximize_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("wmctrl")
            .arg("-i")
            .arg("-r")
            .arg(format!("0x{:x}", id))
            .arg("-b")
            .arg("add,maximized_vert,maximized_horz")
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Maximize a window (Windows).
    #[cfg(target_os = "windows")]
    pub fn maximize_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Maximize a window (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn maximize_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Close a window.
    #[cfg(target_os = "macos")]
    pub fn close_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let script = format!(
            r#"
            tell application "System Events"
                try
                    click (first button of window 1 of process id {} whose subrole is "AXCloseButton")
                end try
            end tell
            "#,
            id
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Close a window.
    #[cfg(target_os = "linux")]
    pub fn close_window(&self, id: u64) -> Result<(), WindowError> {
        use std::process::Command;

        let output = Command::new("wmctrl")
            .arg("-i")
            .arg("-c")
            .arg(format!("0x{:x}", id))
            .output()
            .map_err(|e| WindowError::MoveFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(WindowError::MoveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Close a window (Windows).
    #[cfg(target_os = "windows")]
    pub fn close_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Close a window (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn close_window(&self, _id: u64) -> Result<(), WindowError> {
        Err(WindowError::PlatformNotSupported)
    }
}

impl Default for WindowController {
    fn default() -> Self {
        Self
    }
}

/// Parse AppleScript window list output.
#[cfg(target_os = "macos")]
fn parse_applescript_windows(output: &str) -> Result<Vec<WindowInfo>, WindowError> {
    // AppleScript output format is complex; simplified parsing here
    let mut windows = Vec::new();
    let mut id_counter: u64 = 1;

    // Basic parsing - in production, would need more robust parsing
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try to extract window info from AppleScript output
        // This is a simplified version
        windows.push(WindowInfo {
            id: id_counter,
            title: line.to_string(),
            app_name: "Unknown".to_string(),
            pid: 0,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            is_minimized: false,
            is_maximized: false,
            is_focused: id_counter == 1,
        });
        id_counter += 1;
    }

    Ok(windows)
}

/// Parse wmctrl window list output.
#[cfg(target_os = "linux")]
fn parse_wmctrl_windows(output: &str) -> Result<Vec<WindowInfo>, WindowError> {
    let mut windows = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            continue;
        }

        // wmctrl -l -p -G format:
        // 0x02c00004  0 12345   0    0    1920 1080  hostname Window Title

        let id = u64::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(0);
        let pid = parts[2].parse().unwrap_or(0);
        let x = parts[3].parse().unwrap_or(0);
        let y = parts[4].parse().unwrap_or(0);
        let width = parts[5].parse().unwrap_or(0);
        let height = parts[6].parse().unwrap_or(0);

        // Window title is the rest
        let title = parts[8..].join(" ");

        windows.push(WindowInfo {
            id,
            title,
            app_name: "Unknown".to_string(),
            pid,
            x,
            y,
            width,
            height,
            is_minimized: false,
            is_maximized: false,
            is_focused: false,
        });
    }

    Ok(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_error_display() {
        let err = WindowError::NotFound("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_window_info_serialize() {
        let info = WindowInfo {
            id: 123,
            title: "Test Window".to_string(),
            app_name: "Test App".to_string(),
            pid: 456,
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            is_minimized: false,
            is_maximized: false,
            is_focused: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Test Window"));
        assert!(json.contains("123"));
    }

    #[test]
    fn test_window_controller_new() {
        let controller = WindowController::new();
        assert!(controller.is_ok());
    }

    #[test]
    fn test_window_controller_default() {
        let _controller = WindowController::default();
    }
}
