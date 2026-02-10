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
#[path = "screenshot_tests.rs"]
mod tests;
