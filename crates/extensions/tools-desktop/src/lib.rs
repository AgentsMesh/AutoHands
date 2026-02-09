//! Desktop automation tools for AutoHands.
//!
//! Provides system-level control capabilities:
//!
//! ## Screenshot
//! - `desktop_screenshot` - Capture full screen or region
//! - `desktop_screen_info` - Get monitor information
//!
//! ## Mouse Control
//! - `desktop_mouse_move` - Move cursor to position
//! - `desktop_mouse_click` - Click mouse button
//! - `desktop_mouse_scroll` - Scroll mouse wheel
//!
//! ## Keyboard Control
//! - `desktop_keyboard_type` - Type text
//! - `desktop_keyboard_key` - Press a single key
//! - `desktop_keyboard_hotkey` - Press key combination
//!
//! ## Clipboard
//! - `desktop_clipboard_get` - Get clipboard content
//! - `desktop_clipboard_set` - Set clipboard content
//!
//! ## Window Management
//! - `desktop_window_list` - List all windows
//! - `desktop_window_focus` - Focus a window
//! - `desktop_window_move` - Move a window
//! - `desktop_window_resize` - Resize a window
//! - `desktop_window_minimize` - Minimize a window
//! - `desktop_window_maximize` - Maximize a window
//! - `desktop_window_close` - Close a window
//!
//! ## OCR (Optical Character Recognition)
//! - `desktop_ocr_screen` - Recognize text from the entire screen
//! - `desktop_ocr_region` - Recognize text from a specific region
//! - `desktop_ocr_image` - Recognize text from a base64 encoded image

mod clipboard;
mod extension;
mod input;
mod ocr;
mod ocr_tools;
mod screenshot;
mod tools;
mod window;
mod window_tools;

pub use clipboard::{ClipboardController, ClipboardError};
pub use extension::DesktopToolsExtension;
pub use input::{InputController, InputError, MouseButton};
pub use ocr::{OcrController, OcrError, OcrResult, TextBlock};
pub use ocr_tools::*;
pub use screenshot::{
    capture_region, capture_screen, get_screen_size, list_monitors, MonitorInfo, Screenshot,
    ScreenshotError,
};
pub use tools::*;
pub use window::{WindowController, WindowError, WindowInfo};
pub use window_tools::*;
