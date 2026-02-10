//! WindowController operations: move, resize, minimize, maximize, close.

use super::{WindowController, WindowError};

impl WindowController {
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
            .arg("-i").arg("-r").arg(format!("0x{:x}", id))
            .arg("-e").arg(format!("0,{},{}", x, y))
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
            .arg("-i").arg("-r").arg(format!("0x{:x}", id))
            .arg("-e").arg(format!("0,-1,-1,{},{}", width, height))
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
            .arg("-e").arg(&script).output()
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
            .arg("windowminimize").arg(format!("{}", id))
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
            .arg("-e").arg(&script).output()
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
            .arg("-i").arg("-r").arg(format!("0x{:x}", id))
            .arg("-b").arg("add,maximized_vert,maximized_horz")
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
            .arg("-e").arg(&script).output()
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
            .arg("-i").arg("-c").arg(format!("0x{:x}", id))
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
