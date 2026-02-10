//! Platform-specific output parsers for window list commands.

use super::{WindowError, WindowInfo};

/// Parse AppleScript window list output.
#[cfg(target_os = "macos")]
pub fn parse_applescript_windows(output: &str) -> Result<Vec<WindowInfo>, WindowError> {
    let mut windows = Vec::new();
    let mut id_counter: u64 = 1;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

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
pub fn parse_wmctrl_windows(output: &str) -> Result<Vec<WindowInfo>, WindowError> {
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
