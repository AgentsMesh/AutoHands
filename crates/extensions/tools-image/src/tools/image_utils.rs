//! Shared image utility functions.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat};

use autohands_protocols::error::ToolError;

pub fn load_image(path: &str) -> Result<DynamicImage, ToolError> {
    image::open(path).map_err(|e| ToolError::ExecutionFailed(format!("Failed to load image: {}", e)))
}

pub fn save_image(img: &DynamicImage, path: &str, format: Option<&str>) -> Result<(), ToolError> {
    let format = format
        .map(|f| parse_format(f))
        .transpose()?
        .or_else(|| ImageFormat::from_path(path).ok());

    match format {
        Some(fmt) => img
            .save_with_format(path, fmt)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save image: {}", e))),
        None => img
            .save(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save image: {}", e))),
    }
}

pub fn parse_format(format: &str) -> Result<ImageFormat, ToolError> {
    match format.to_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        "gif" => Ok(ImageFormat::Gif),
        "bmp" => Ok(ImageFormat::Bmp),
        "webp" => Ok(ImageFormat::WebP),
        "tiff" | "tif" => Ok(ImageFormat::Tiff),
        "ico" => Ok(ImageFormat::Ico),
        _ => Err(ToolError::ExecutionFailed(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}

pub fn image_to_base64(img: &DynamicImage, format: ImageFormat) -> Result<String, ToolError> {
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, format)
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to encode image: {}", e)))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(buffer.into_inner()))
}
