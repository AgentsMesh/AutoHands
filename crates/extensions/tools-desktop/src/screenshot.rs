//! Screenshot capture utilities.

use std::io::Cursor;

use screenshots::image::ImageOutputFormat;
use screenshots::Screen;
use thiserror::Error;

/// Screenshot errors.
#[derive(Debug, Error)]
pub enum ScreenshotError {
    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

    #[error("No monitor found")]
    NoMonitor,
}

/// Screenshot result.
#[derive(Debug)]
pub struct Screenshot {
    /// PNG image data.
    pub data: Vec<u8>,
    /// Image width.
    pub width: u32,
    /// Image height.
    pub height: u32,
}

impl Screenshot {
    /// Encode as base64.
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }

    /// Get PNG data.
    pub fn to_png(&self) -> Vec<u8> {
        self.data.clone()
    }
}

/// Capture the entire screen (primary monitor).
pub fn capture_screen() -> Result<Screenshot, ScreenshotError> {
    let screens = Screen::all().map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    let screen = screens
        .into_iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| Screen::all().ok()?.into_iter().next())
        .ok_or(ScreenshotError::NoMonitor)?;

    capture_screen_impl(&screen)
}

fn capture_screen_impl(screen: &Screen) -> Result<Screenshot, ScreenshotError> {
    let image = screen
        .capture()
        .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    let width = image.width();
    let height = image.height();

    // Encode to PNG
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, ImageOutputFormat::Png)
        .map_err(|e| ScreenshotError::EncodingFailed(e.to_string()))?;

    Ok(Screenshot {
        data: buffer.into_inner(),
        width,
        height,
    })
}

/// Capture a region of the screen.
pub fn capture_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<Screenshot, ScreenshotError> {
    let screens = Screen::all().map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    let screen = screens
        .into_iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| Screen::all().ok()?.into_iter().next())
        .ok_or(ScreenshotError::NoMonitor)?;

    let image = screen
        .capture_area(x, y, width, height)
        .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    let img_width = image.width();
    let img_height = image.height();

    // Encode to PNG
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, ImageOutputFormat::Png)
        .map_err(|e| ScreenshotError::EncodingFailed(e.to_string()))?;

    Ok(Screenshot {
        data: buffer.into_inner(),
        width: img_width,
        height: img_height,
    })
}

/// Get screen dimensions.
pub fn get_screen_size() -> Result<(u32, u32), ScreenshotError> {
    let screens = Screen::all().map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    let screen = screens
        .into_iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| Screen::all().ok()?.into_iter().next())
        .ok_or(ScreenshotError::NoMonitor)?;

    Ok((screen.display_info.width, screen.display_info.height))
}

/// List all monitors.
pub fn list_monitors() -> Result<Vec<MonitorInfo>, ScreenshotError> {
    let screens = Screen::all().map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

    Ok(screens
        .into_iter()
        .enumerate()
        .map(|(idx, s)| MonitorInfo {
            id: s.display_info.id,
            name: format!("Monitor {}", idx + 1),
            x: s.display_info.x,
            y: s.display_info.y,
            width: s.display_info.width,
            height: s.display_info.height,
            is_primary: s.display_info.is_primary,
            scale_factor: s.display_info.scale_factor,
        })
        .collect())
}

/// Monitor information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub scale_factor: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_info_serialize() {
        let info = MonitorInfo {
            id: 1,
            name: "Main".to_string(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
            scale_factor: 1.0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Main"));
        assert!(json.contains("1920"));
        assert!(json.contains("1080"));
        assert!(json.contains("is_primary"));
    }

    #[test]
    fn test_monitor_info_fields() {
        let info = MonitorInfo {
            id: 2,
            name: "Secondary".to_string(),
            x: 1920,
            y: 0,
            width: 1280,
            height: 720,
            is_primary: false,
            scale_factor: 1.5,
        };
        assert_eq!(info.id, 2);
        assert_eq!(info.name, "Secondary");
        assert_eq!(info.x, 1920);
        assert_eq!(info.width, 1280);
        assert!(!info.is_primary);
        assert_eq!(info.scale_factor, 1.5);
    }

    #[test]
    fn test_screenshot_to_base64() {
        let screenshot = Screenshot {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes
            width: 100,
            height: 100,
        };
        let base64 = screenshot.to_base64();
        assert!(!base64.is_empty());
        // Verify it's valid base64
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&base64);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), vec![0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_screenshot_error_display() {
        let err = ScreenshotError::CaptureFailed("test error".to_string());
        assert_eq!(err.to_string(), "Capture failed: test error");

        let err = ScreenshotError::EncodingFailed("encoding error".to_string());
        assert_eq!(err.to_string(), "Encoding failed: encoding error");

        let err = ScreenshotError::NoMonitor;
        assert_eq!(err.to_string(), "No monitor found");
    }

    #[test]
    fn test_screenshot_error_debug() {
        let err = ScreenshotError::CaptureFailed("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("CaptureFailed"));

        let err = ScreenshotError::EncodingFailed("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("EncodingFailed"));

        let err = ScreenshotError::NoMonitor;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NoMonitor"));
    }

    #[test]
    fn test_monitor_info_debug() {
        let info = MonitorInfo {
            id: 1,
            name: "Test".to_string(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
            scale_factor: 1.0,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("MonitorInfo"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn test_monitor_info_clone() {
        let info = MonitorInfo {
            id: 1,
            name: "Test".to_string(),
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            is_primary: true,
            scale_factor: 2.0,
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.name, info.name);
        assert_eq!(cloned.x, info.x);
        assert_eq!(cloned.y, info.y);
        assert_eq!(cloned.width, info.width);
        assert_eq!(cloned.height, info.height);
        assert_eq!(cloned.is_primary, info.is_primary);
        assert_eq!(cloned.scale_factor, info.scale_factor);
    }

    #[test]
    fn test_screenshot_debug() {
        let screenshot = Screenshot {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        };
        let debug = format!("{:?}", screenshot);
        assert!(debug.contains("Screenshot"));
    }

    #[test]
    fn test_screenshot_empty_data() {
        let screenshot = Screenshot {
            data: vec![],
            width: 0,
            height: 0,
        };
        let base64 = screenshot.to_base64();
        assert!(base64.is_empty());
    }

    #[test]
    fn test_screenshot_large_data() {
        let data = vec![0u8; 10000];
        let screenshot = Screenshot {
            data,
            width: 100,
            height: 100,
        };
        let base64 = screenshot.to_base64();
        assert!(!base64.is_empty());
        // Verify round trip
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&base64).unwrap();
        assert_eq!(decoded.len(), 10000);
    }

    #[test]
    fn test_monitor_info_negative_coordinates() {
        let info = MonitorInfo {
            id: 1,
            name: "Negative".to_string(),
            x: -1920,
            y: -100,
            width: 1920,
            height: 1080,
            is_primary: false,
            scale_factor: 1.0,
        };
        assert_eq!(info.x, -1920);
        assert_eq!(info.y, -100);
    }

    #[test]
    fn test_monitor_info_serialization_all_fields() {
        let info = MonitorInfo {
            id: 5,
            name: "Ultra Wide".to_string(),
            x: 2560,
            y: 0,
            width: 3440,
            height: 1440,
            is_primary: false,
            scale_factor: 1.25,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":5"));
        assert!(json.contains("Ultra Wide"));
        assert!(json.contains("\"x\":2560"));
        assert!(json.contains("\"y\":0"));
        assert!(json.contains("\"width\":3440"));
        assert!(json.contains("\"height\":1440"));
        assert!(json.contains("\"is_primary\":false"));
        assert!(json.contains("1.25"));
    }

    // Integration tests that require actual screen access
    #[test]
    #[ignore] // Requires actual display
    fn test_capture_screen() {
        let result = capture_screen();
        assert!(result.is_ok());
        let screenshot = result.unwrap();
        assert!(screenshot.width > 0);
        assert!(screenshot.height > 0);
        assert!(!screenshot.data.is_empty());
    }

    #[test]
    #[ignore] // Requires actual display
    fn test_capture_region() {
        let result = capture_region(0, 0, 100, 100);
        assert!(result.is_ok());
        let screenshot = result.unwrap();
        assert!(screenshot.width <= 100);
        assert!(screenshot.height <= 100);
    }

    #[test]
    #[ignore] // Requires actual display
    fn test_get_screen_size() {
        let result = get_screen_size();
        assert!(result.is_ok());
        let (width, height) = result.unwrap();
        assert!(width > 0);
        assert!(height > 0);
    }

    #[test]
    #[ignore] // Requires actual display
    fn test_list_monitors() {
        let result = list_monitors();
        assert!(result.is_ok());
        let monitors = result.unwrap();
        assert!(!monitors.is_empty());
        // At least one should be primary
        assert!(monitors.iter().any(|m| m.is_primary));
    }
}
