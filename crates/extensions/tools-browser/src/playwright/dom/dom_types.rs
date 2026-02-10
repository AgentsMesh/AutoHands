//! DOM type definitions: ViewportInfo, BoundingBox, NodeAttributes, EnhancedNode.

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

/// Enhanced DOM node with merged information from multiple CDP trees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedNode {
    /// Unique identifier for this node in the page.
    pub id: String,
    /// Backend node ID from CDP.
    pub backend_node_id: i64,
    /// Tag name (lowercase).
    pub tag_name: String,
    /// Node attributes.
    pub attributes: NodeAttributes,
    /// Text content (direct text only, not from children).
    pub text_content: String,
    /// Bounding box in viewport coordinates.
    pub bounding_box: BoundingBox,
    /// Whether the element is visible.
    pub is_visible: bool,
    /// Whether the element is in the current viewport.
    pub is_in_viewport: bool,
    /// Clickability score (0.0 - 1.0).
    pub clickability_score: f64,
    /// Reasons why this element is considered clickable.
    #[serde(default)]
    pub clickability_reasons: Vec<String>,
    /// Paint order for z-index handling (higher = on top).
    pub paint_order: i32,
    /// Whether this is an interactive element.
    pub is_interactive: bool,
    /// Whether this element can receive focus.
    pub is_focusable: bool,
    /// Parent node ID.
    pub parent_id: Option<String>,
    /// Child node IDs.
    #[serde(default)]
    pub children: Vec<String>,
    /// XPath selector for this element.
    pub xpath: String,
    /// CSS selector for this element.
    pub css_selector: String,
    /// Computed styles relevant for interaction.
    #[serde(default)]
    pub computed_styles: HashMap<String, String>,
}
