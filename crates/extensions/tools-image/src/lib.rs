//! Image processing tools for AutoHands.
//!
//! Provides tools for image manipulation and analysis:
//!
//! ## Tools
//!
//! - `image_resize` - Resize images to specified dimensions
//! - `image_crop` - Crop a region from an image
//! - `image_convert` - Convert between image formats
//! - `image_info` - Get image metadata (dimensions, format, etc.)
//! - `image_rotate` - Rotate images by degrees
//! - `image_flip` - Flip images horizontally or vertically

mod extension;
mod tools;

pub use extension::ImageToolsExtension;
pub use tools::*;
