//! WindowController core: new/default, list_windows, focus_window.

use super::{WindowError, WindowInfo};

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
        super::parsers::parse_applescript_windows(&stdout)
    }

    /// List all windows.
    #[cfg(target_os = "linux")]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
        use std::process::Command;

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
        super::parsers::parse_wmctrl_windows(&stdout)
    }

    /// List all windows.
    #[cfg(target_os = "windows")]
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, WindowError> {
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
}

impl Default for WindowController {
    fn default() -> Self {
        Self
    }
}
