//! Clipboard operations.

use arboard::Clipboard;
use screenshots::image::{self, ImageOutputFormat};
use thiserror::Error;

/// Clipboard errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    #[error("Clipboard access failed: {0}")]
    AccessFailed(String),

    #[error("No text in clipboard")]
    NoText,

    #[error("No image in clipboard")]
    NoImage,
}

/// Clipboard controller.
pub struct ClipboardController {
    clipboard: Clipboard,
}

impl ClipboardController {
    /// Create a new clipboard controller.
    pub fn new() -> Result<Self, ClipboardError> {
        let clipboard =
            Clipboard::new().map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;
        Ok(Self { clipboard })
    }

    /// Get text from clipboard.
    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.clipboard
            .get_text()
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))
    }

    /// Set text to clipboard.
    pub fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.clipboard
            .set_text(text)
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))
    }

    /// Get image from clipboard as PNG bytes.
    pub fn get_image(&mut self) -> Result<Vec<u8>, ClipboardError> {
        let image = self
            .clipboard
            .get_image()
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;

        // Convert to PNG
        let img = image::RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .ok_or(ClipboardError::NoImage)?;

        let mut buffer = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageOutputFormat::Png)
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;

        Ok(buffer.into_inner())
    }

    /// Set image to clipboard from PNG bytes.
    pub fn set_image(&mut self, png_data: &[u8]) -> Result<(), ClipboardError> {
        let img = image::load_from_memory(png_data)
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?
            .to_rgba8();

        let image_data = arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Owned(img.into_raw()),
        };

        self.clipboard
            .set_image(image_data)
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))
    }

    /// Clear clipboard.
    pub fn clear(&mut self) -> Result<(), ClipboardError> {
        self.clipboard
            .clear()
            .map_err(|e| ClipboardError::AccessFailed(e.to_string()))
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
