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

/// Trait for clipboard operations (allows mocking in tests).
#[allow(dead_code)]
pub trait ClipboardBackend {
    /// Get text from clipboard.
    fn get_text(&mut self) -> Result<String, ClipboardError>;
    /// Set text to clipboard.
    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError>;
    /// Get image from clipboard.
    fn get_image(&mut self) -> Result<Vec<u8>, ClipboardError>;
    /// Set image to clipboard.
    fn set_image(&mut self, png_data: &[u8]) -> Result<(), ClipboardError>;
    /// Clear clipboard.
    fn clear(&mut self) -> Result<(), ClipboardError>;
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

impl ClipboardBackend for ClipboardController {
    fn get_text(&mut self) -> Result<String, ClipboardError> {
        ClipboardController::get_text(self)
    }

    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        ClipboardController::set_text(self, text)
    }

    fn get_image(&mut self) -> Result<Vec<u8>, ClipboardError> {
        ClipboardController::get_image(self)
    }

    fn set_image(&mut self, png_data: &[u8]) -> Result<(), ClipboardError> {
        ClipboardController::set_image(self, png_data)
    }

    fn clear(&mut self) -> Result<(), ClipboardError> {
        ClipboardController::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock clipboard backend for testing.
    struct MockClipboard {
        text: Option<String>,
        image: Option<Vec<u8>>,
        fail_on_get: bool,
        fail_on_set: bool,
    }

    impl MockClipboard {
        fn new() -> Self {
            Self {
                text: None,
                image: None,
                fail_on_get: false,
                fail_on_set: false,
            }
        }

        fn with_text(mut self, text: &str) -> Self {
            self.text = Some(text.to_string());
            self
        }

        fn with_image(mut self, data: Vec<u8>) -> Self {
            self.image = Some(data);
            self
        }

        fn fail_on_get(mut self) -> Self {
            self.fail_on_get = true;
            self
        }

        fn fail_on_set(mut self) -> Self {
            self.fail_on_set = true;
            self
        }
    }

    impl ClipboardBackend for MockClipboard {
        fn get_text(&mut self) -> Result<String, ClipboardError> {
            if self.fail_on_get {
                return Err(ClipboardError::AccessFailed("mock error".to_string()));
            }
            self.text.clone().ok_or(ClipboardError::NoText)
        }

        fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
            if self.fail_on_set {
                return Err(ClipboardError::AccessFailed("mock error".to_string()));
            }
            self.text = Some(text.to_string());
            Ok(())
        }

        fn get_image(&mut self) -> Result<Vec<u8>, ClipboardError> {
            if self.fail_on_get {
                return Err(ClipboardError::AccessFailed("mock error".to_string()));
            }
            self.image.clone().ok_or(ClipboardError::NoImage)
        }

        fn set_image(&mut self, png_data: &[u8]) -> Result<(), ClipboardError> {
            if self.fail_on_set {
                return Err(ClipboardError::AccessFailed("mock error".to_string()));
            }
            self.image = Some(png_data.to_vec());
            Ok(())
        }

        fn clear(&mut self) -> Result<(), ClipboardError> {
            if self.fail_on_set {
                return Err(ClipboardError::AccessFailed("mock error".to_string()));
            }
            self.text = None;
            self.image = None;
            Ok(())
        }
    }

    // Tests using mock clipboard
    #[test]
    fn test_mock_clipboard_get_text() {
        let mut clipboard = MockClipboard::new().with_text("hello");
        assert_eq!(clipboard.get_text().unwrap(), "hello");
    }

    #[test]
    fn test_mock_clipboard_get_text_empty() {
        let mut clipboard = MockClipboard::new();
        let result = clipboard.get_text();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ClipboardError::NoText);
    }

    #[test]
    fn test_mock_clipboard_set_text() {
        let mut clipboard = MockClipboard::new();
        clipboard.set_text("world").unwrap();
        assert_eq!(clipboard.get_text().unwrap(), "world");
    }

    #[test]
    fn test_mock_clipboard_get_image() {
        let data = vec![0x89, 0x50, 0x4E, 0x47];
        let mut clipboard = MockClipboard::new().with_image(data.clone());
        assert_eq!(clipboard.get_image().unwrap(), data);
    }

    #[test]
    fn test_mock_clipboard_get_image_empty() {
        let mut clipboard = MockClipboard::new();
        let result = clipboard.get_image();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ClipboardError::NoImage);
    }

    #[test]
    fn test_mock_clipboard_set_image() {
        let mut clipboard = MockClipboard::new();
        let data = vec![1, 2, 3, 4];
        clipboard.set_image(&data).unwrap();
        assert_eq!(clipboard.get_image().unwrap(), data);
    }

    #[test]
    fn test_mock_clipboard_clear() {
        let mut clipboard = MockClipboard::new()
            .with_text("text")
            .with_image(vec![1, 2, 3]);
        clipboard.clear().unwrap();
        assert!(clipboard.get_text().is_err());
        assert!(clipboard.get_image().is_err());
    }

    #[test]
    fn test_mock_clipboard_get_text_fails() {
        let mut clipboard = MockClipboard::new().with_text("text").fail_on_get();
        let result = clipboard.get_text();
        assert!(result.is_err());
        match result.unwrap_err() {
            ClipboardError::AccessFailed(msg) => assert!(msg.contains("mock error")),
            _ => panic!("Expected AccessFailed"),
        }
    }

    #[test]
    fn test_mock_clipboard_set_text_fails() {
        let mut clipboard = MockClipboard::new().fail_on_set();
        let result = clipboard.set_text("text");
        assert!(result.is_err());
        match result.unwrap_err() {
            ClipboardError::AccessFailed(msg) => assert!(msg.contains("mock error")),
            _ => panic!("Expected AccessFailed"),
        }
    }

    #[test]
    fn test_mock_clipboard_get_image_fails() {
        let mut clipboard = MockClipboard::new().with_image(vec![1]).fail_on_get();
        let result = clipboard.get_image();
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_clipboard_set_image_fails() {
        let mut clipboard = MockClipboard::new().fail_on_set();
        let result = clipboard.set_image(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_clipboard_clear_fails() {
        let mut clipboard = MockClipboard::new().fail_on_set();
        let result = clipboard.clear();
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_clipboard_unicode() {
        let mut clipboard = MockClipboard::new();
        clipboard.set_text("こんにちは 🎉").unwrap();
        assert_eq!(clipboard.get_text().unwrap(), "こんにちは 🎉");
    }

    #[test]
    fn test_mock_clipboard_empty_string() {
        let mut clipboard = MockClipboard::new();
        clipboard.set_text("").unwrap();
        assert_eq!(clipboard.get_text().unwrap(), "");
    }

    #[test]
    fn test_mock_clipboard_overwrite() {
        let mut clipboard = MockClipboard::new().with_text("old");
        clipboard.set_text("new").unwrap();
        assert_eq!(clipboard.get_text().unwrap(), "new");
    }

    // Error tests
    #[test]
    fn test_clipboard_error_no_text_display() {
        let err = ClipboardError::NoText;
        assert_eq!(err.to_string(), "No text in clipboard");
    }

    #[test]
    fn test_clipboard_error_no_image_display() {
        let err = ClipboardError::NoImage;
        assert_eq!(err.to_string(), "No image in clipboard");
    }

    #[test]
    fn test_clipboard_error_access_failed_display() {
        let err = ClipboardError::AccessFailed("permission denied".to_string());
        assert_eq!(err.to_string(), "Clipboard access failed: permission denied");
    }

    #[test]
    fn test_clipboard_error_debug() {
        let err = ClipboardError::NoText;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NoText"));
    }

    #[test]
    fn test_clipboard_error_debug_no_image() {
        let err = ClipboardError::NoImage;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NoImage"));
    }

    #[test]
    fn test_clipboard_error_debug_access_failed() {
        let err = ClipboardError::AccessFailed("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("AccessFailed"));
    }

    #[test]
    fn test_clipboard_error_access_failed_empty_message() {
        let err = ClipboardError::AccessFailed(String::new());
        assert_eq!(err.to_string(), "Clipboard access failed: ");
    }

    #[test]
    fn test_clipboard_error_access_failed_long_message() {
        let long_msg = "a".repeat(1000);
        let err = ClipboardError::AccessFailed(long_msg.clone());
        assert!(err.to_string().contains(&long_msg));
    }

    // Integration tests that require actual clipboard access
    #[test]
    #[ignore] // Requires clipboard access
    fn test_clipboard_set_and_get_text() {
        let mut controller = ClipboardController::new().unwrap();
        controller.set_text("test clipboard content").unwrap();
        let text = controller.get_text().unwrap();
        assert_eq!(text, "test clipboard content");
    }

    #[test]
    #[ignore] // Requires clipboard access
    fn test_clipboard_clear() {
        let mut controller = ClipboardController::new().unwrap();
        controller.set_text("some text").unwrap();
        controller.clear().unwrap();
    }

    #[test]
    #[ignore] // Requires clipboard access
    fn test_clipboard_set_empty_text() {
        let mut controller = ClipboardController::new().unwrap();
        controller.set_text("").unwrap();
        let text = controller.get_text().unwrap();
        assert!(text.is_empty());
    }

    #[test]
    #[ignore] // Requires clipboard access
    fn test_clipboard_set_unicode_text() {
        let mut controller = ClipboardController::new().unwrap();
        controller.set_text("你好世界 🌍").unwrap();
        let text = controller.get_text().unwrap();
        assert_eq!(text, "你好世界 🌍");
    }
}
