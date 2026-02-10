//! Shared DOM types: viewport, bounding box, and node attributes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Viewport information for coordinate calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportInfo {
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height in pixels.
    pub height: u32,
    /// Device pixel ratio.
    pub device_pixel_ratio: f64,
    /// Scroll X offset.
    pub scroll_x: f64,
    /// Scroll Y offset.
    pub scroll_y: f64,
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            device_pixel_ratio: 1.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }
}

/// Bounding box for an element.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    /// Check if a point is inside this bounding box.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// Get the center point of this bounding box.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Check if this box intersects with another.
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Check if this box is visible in viewport.
    pub fn is_visible_in_viewport(&self, viewport: &ViewportInfo) -> bool {
        let vp_box = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: viewport.width as f64,
            height: viewport.height as f64,
        };
        self.intersects(&vp_box)
    }
}

/// Node attributes extracted from DOM.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeAttributes {
    /// Element ID attribute.
    pub id: Option<String>,
    /// Element class names.
    pub class: Option<String>,
    /// Href for links.
    pub href: Option<String>,
    /// Src for images/iframes.
    pub src: Option<String>,
    /// Alt text.
    pub alt: Option<String>,
    /// Title attribute.
    pub title: Option<String>,
    /// Placeholder text.
    pub placeholder: Option<String>,
    /// Value for inputs.
    pub value: Option<String>,
    /// Type attribute.
    pub r#type: Option<String>,
    /// Name attribute.
    pub name: Option<String>,
    /// Role attribute (ARIA).
    pub role: Option<String>,
    /// Aria-label.
    pub aria_label: Option<String>,
    /// Aria-expanded.
    pub aria_expanded: Option<String>,
    /// Aria-selected.
    pub aria_selected: Option<String>,
    /// Data attributes.
    #[serde(default)]
    pub data: HashMap<String, String>,
}
