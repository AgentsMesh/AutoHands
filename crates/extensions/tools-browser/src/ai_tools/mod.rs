//! AI-powered browser automation tools.
//!
//! These tools use vision LLM capabilities to interact with web pages
//! based on natural language descriptions rather than CSS selectors.

mod click;
mod extract;
mod fill;
mod vision_provider;

pub use click::*;
pub use extract::*;
pub use fill::*;
pub use vision_provider::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;

/// Coordinates of an element found by AI vision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementCoordinates {
    pub x: i32,
    pub y: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub confidence: f32,
}

/// Parse coordinates from a vision model response.
pub(crate) fn parse_coordinates(response: &str) -> Result<ElementCoordinates, ToolError> {
    // Try to parse JSON response first
    if let Ok(coords) = serde_json::from_str::<ElementCoordinates>(response) {
        return Ok(coords);
    }

    // Try to extract coordinates from text response
    let patterns = [
        r"x[:\s]*(\d+)[,\s]+y[:\s]*(\d+)",
        r"\((\d+)[,\s]+(\d+)\)",
        r"coordinates[:\s]*(\d+)[,\s]+(\d+)",
        r"position[:\s]*(\d+)[,\s]+(\d+)",
        r"(\d+)[,\s]+(\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(response) {
                if let (Some(x), Some(y)) = (caps.get(1), caps.get(2)) {
                    if let (Ok(x_val), Ok(y_val)) = (x.as_str().parse(), y.as_str().parse()) {
                        return Ok(ElementCoordinates {
                            x: x_val,
                            y: y_val,
                            width: None,
                            height: None,
                            confidence: 0.8,
                        });
                    }
                }
            }
        }
    }

    Err(ToolError::ExecutionFailed(format!(
        "Could not parse coordinates from response: {}",
        response
    )))
}
