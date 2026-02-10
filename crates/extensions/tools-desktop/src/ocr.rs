//! OCR (Optical Character Recognition) functionality.
//!
//! Provides text recognition from screenshots and images.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors related to OCR operations.
#[derive(Debug, Error)]
pub enum OcrError {
    /// Platform not supported.
    #[error("OCR not supported on this platform")]
    PlatformNotSupported,

    /// Failed to perform OCR.
    #[error("OCR failed: {0}")]
    RecognitionFailed(String),

    /// Invalid image data.
    #[error("Invalid image data: {0}")]
    InvalidImage(String),

    /// OCR engine not available.
    #[error("OCR engine not available: {0}")]
    EngineNotAvailable(String),
}

/// Result of OCR recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// Recognized text.
    pub text: String,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// Individual text blocks with positions.
    pub blocks: Vec<TextBlock>,
}

/// A block of recognized text with position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// The recognized text.
    pub text: String,

    /// X position.
    pub x: i32,

    /// Y position.
    pub y: i32,

    /// Width of the text block.
    pub width: u32,

    /// Height of the text block.
    pub height: u32,

    /// Confidence score for this block.
    pub confidence: f32,
}

/// OCR controller for text recognition.
pub struct OcrController;

impl OcrController {
    /// Create a new OCR controller.
    pub fn new() -> Result<Self, OcrError> {
        Ok(Self)
    }

    /// Recognize text from image data (PNG format).
    #[cfg(target_os = "macos")]
    pub fn recognize_image(&self, image_data: &[u8]) -> Result<OcrResult, OcrError> {
        use std::process::Command;

        // Write image to temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("ocr_input_{}.png", std::process::id()));

        std::fs::write(&temp_file, image_data)
            .map_err(|e| OcrError::InvalidImage(e.to_string()))?;

        // Use macOS Vision framework via Swift script
        let script = format!(
            r#"
            import Vision
            import AppKit
            import Foundation

            let imagePath = "{}"
            guard let image = NSImage(contentsOfFile: imagePath) else {{
                print("ERROR: Could not load image")
                exit(1)
            }}

            guard let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {{
                print("ERROR: Could not convert to CGImage")
                exit(1)
            }}

            let request = VNRecognizeTextRequest {{ request, error in
                guard let observations = request.results as? [VNRecognizedTextObservation] else {{
                    print("ERROR: No results")
                    return
                }}

                var output: [[String: Any]] = []
                for observation in observations {{
                    if let candidate = observation.topCandidates(1).first {{
                        let box = observation.boundingBox
                        output.append([
                            "text": candidate.string,
                            "confidence": candidate.confidence,
                            "x": Int(box.origin.x * CGFloat(cgImage.width)),
                            "y": Int((1 - box.origin.y - box.height) * CGFloat(cgImage.height)),
                            "width": Int(box.width * CGFloat(cgImage.width)),
                            "height": Int(box.height * CGFloat(cgImage.height))
                        ])
                    }}
                }}

                if let jsonData = try? JSONSerialization.data(withJSONObject: output, options: []),
                   let jsonString = String(data: jsonData, encoding: .utf8) {{
                    print(jsonString)
                }}
            }}

            request.recognitionLevel = .accurate
            request.usesLanguageCorrection = true

            let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
            try? handler.perform([request])
            "#,
            temp_file.display()
        );

        // Write Swift script to temp file
        let script_file = temp_dir.join(format!("ocr_script_{}.swift", std::process::id()));
        std::fs::write(&script_file, &script)
            .map_err(|e| OcrError::RecognitionFailed(e.to_string()))?;

        // Execute Swift script
        let output = Command::new("swift")
            .arg(&script_file)
            .output()
            .map_err(|e| OcrError::RecognitionFailed(e.to_string()))?;

        // Clean up temp files
        let _ = std::fs::remove_file(&temp_file);
        let _ = std::fs::remove_file(&script_file);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OcrError::RecognitionFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_vision_output(&stdout)
    }

    /// Recognize text from image data (Linux - uses Tesseract if available).
    #[cfg(target_os = "linux")]
    pub fn recognize_image(&self, image_data: &[u8]) -> Result<OcrResult, OcrError> {
        use std::process::Command;

        // Check if tesseract is available
        let check = Command::new("which")
            .arg("tesseract")
            .output()
            .map_err(|e| OcrError::RecognitionFailed(e.to_string()))?;

        if !check.status.success() {
            return Err(OcrError::EngineNotAvailable(
                "Tesseract OCR is not installed. Install with: apt install tesseract-ocr"
                    .to_string(),
            ));
        }

        // Write image to temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("ocr_input_{}.png", std::process::id()));
        let output_file = temp_dir.join(format!("ocr_output_{}", std::process::id()));

        std::fs::write(&temp_file, image_data)
            .map_err(|e| OcrError::InvalidImage(e.to_string()))?;

        // Run Tesseract
        let output = Command::new("tesseract")
            .arg(&temp_file)
            .arg(&output_file)
            .arg("-l")
            .arg("eng+chi_sim") // English + Simplified Chinese
            .arg("--psm")
            .arg("3") // Fully automatic page segmentation
            .output()
            .map_err(|e| OcrError::RecognitionFailed(e.to_string()))?;

        // Read output
        let output_path = format!("{}.txt", output_file.display());
        let text = std::fs::read_to_string(&output_path).unwrap_or_default();

        // Clean up
        let _ = std::fs::remove_file(&temp_file);
        let _ = std::fs::remove_file(&output_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OcrError::RecognitionFailed(stderr.to_string()));
        }

        Ok(OcrResult {
            text: text.trim().to_string(),
            confidence: 0.8, // Tesseract CLI doesn't provide confidence easily
            blocks: vec![TextBlock {
                text: text.trim().to_string(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                confidence: 0.8,
            }],
        })
    }

    /// Recognize text from image data (Windows).
    #[cfg(target_os = "windows")]
    pub fn recognize_image(&self, _image_data: &[u8]) -> Result<OcrResult, OcrError> {
        // Windows would use Windows.Media.Ocr API
        // For now, return not supported
        Err(OcrError::PlatformNotSupported)
    }

    /// Recognize text from image data (unsupported platform).
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub fn recognize_image(&self, _image_data: &[u8]) -> Result<OcrResult, OcrError> {
        Err(OcrError::PlatformNotSupported)
    }

    /// Recognize text from a screenshot of the entire screen.
    pub fn recognize_screen(&self) -> Result<OcrResult, OcrError> {
        let screenshot = crate::screenshot::capture_screen()
            .map_err(|e| OcrError::InvalidImage(e.to_string()))?;

        self.recognize_image(&screenshot.to_png())
    }

    /// Recognize text from a region of the screen.
    pub fn recognize_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<OcrResult, OcrError> {
        let screenshot = crate::screenshot::capture_region(x, y, width, height)
            .map_err(|e| OcrError::InvalidImage(e.to_string()))?;

        self.recognize_image(&screenshot.to_png())
    }
}

impl Default for OcrController {
    fn default() -> Self {
        Self
    }
}

/// Parse macOS Vision framework output.
#[cfg(target_os = "macos")]
fn parse_vision_output(output: &str) -> Result<OcrResult, OcrError> {
    let output = output.trim();

    if output.is_empty() || output.starts_with("ERROR:") {
        return Ok(OcrResult {
            text: String::new(),
            confidence: 0.0,
            blocks: Vec::new(),
        });
    }

    // Parse JSON output
    let blocks: Vec<serde_json::Value> = serde_json::from_str(output)
        .map_err(|e| OcrError::RecognitionFailed(format!("Failed to parse output: {}", e)))?;

    let mut text_parts = Vec::new();
    let mut parsed_blocks = Vec::new();
    let mut total_confidence = 0.0;

    for block in &blocks {
        let text = block["text"].as_str().unwrap_or("");
        let confidence = block["confidence"].as_f64().unwrap_or(0.0) as f32;
        let x = block["x"].as_i64().unwrap_or(0) as i32;
        let y = block["y"].as_i64().unwrap_or(0) as i32;
        let width = block["width"].as_i64().unwrap_or(0) as u32;
        let height = block["height"].as_i64().unwrap_or(0) as u32;

        text_parts.push(text.to_string());
        total_confidence += confidence;

        parsed_blocks.push(TextBlock {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            confidence,
        });
    }

    let avg_confidence = if parsed_blocks.is_empty() {
        0.0
    } else {
        total_confidence / parsed_blocks.len() as f32
    };

    Ok(OcrResult {
        text: text_parts.join("\n"),
        confidence: avg_confidence,
        blocks: parsed_blocks,
    })
}

#[cfg(test)]
#[path = "ocr_tests.rs"]
mod tests;
