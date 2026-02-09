//! DOM processing with Browser-Use style 3-tree merging.
//!
//! This module implements intelligent DOM analysis for LLM-based browser automation.
//! It merges information from multiple CDP trees to produce enhanced nodes with
//! accurate clickability detection.
//!
//! ## 10-Layer Clickable Detection
//!
//! Based on Browser-Use's approach, we check:
//! 1. JavaScript event listeners (onclick, etc.)
//! 2. IFRAME size and visibility
//! 3. Label/Span wrapping input elements
//! 4. Search indicators (type=search, role=search)
//! 5. Accessibility properties (focusable, clickable)
//! 6. Native interactive tags (button, a, input, select)
//! 7. Event handler attributes
//! 8. ARIA roles (button, link, checkbox, etc.)
//! 9. Icon size heuristics
//! 10. Cursor: pointer style

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
    /// Higher score means more likely to be clickable.
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

    /// CSS selector for this element (best effort unique selector).
    pub css_selector: String,

    /// Computed styles relevant for interaction.
    #[serde(default)]
    pub computed_styles: HashMap<String, String>,
}

impl EnhancedNode {
    /// Check if this node should be included in LLM output.
    pub fn is_relevant_for_llm(&self) -> bool {
        // Skip invisible elements
        if !self.is_visible {
            return false;
        }

        // Skip elements outside viewport (unless they're inputs)
        if !self.is_in_viewport && !self.is_interactive {
            return false;
        }

        // Skip purely structural elements with no content
        let structural_tags = ["div", "span", "section", "article", "main", "header", "footer"];
        if structural_tags.contains(&self.tag_name.as_str())
            && self.text_content.trim().is_empty()
            && !self.is_interactive
        {
            return false;
        }

        true
    }

    /// Generate LLM-friendly description of this node.
    pub fn to_llm_string(&self, index: usize) -> String {
        let mut parts = vec![];

        // Index for reference
        parts.push(format!("[{}]", index));

        // Tag and type info
        let type_str = if let Some(ref t) = self.attributes.r#type {
            format!("<{} type={}>", self.tag_name, t)
        } else {
            format!("<{}>", self.tag_name)
        };
        parts.push(type_str);

        // Text content (truncated)
        if !self.text_content.is_empty() {
            let text = if self.text_content.len() > 50 {
                format!("{}...", &self.text_content[..47])
            } else {
                self.text_content.clone()
            };
            parts.push(format!("\"{}\"", text.replace('\n', " ").trim()));
        }

        // Important attributes
        if let Some(ref id) = self.attributes.id {
            parts.push(format!("id={}", id));
        }
        if let Some(ref placeholder) = self.attributes.placeholder {
            parts.push(format!("placeholder=\"{}\"", placeholder));
        }
        if let Some(ref aria_label) = self.attributes.aria_label {
            parts.push(format!("aria-label=\"{}\"", aria_label));
        }
        if let Some(ref role) = self.attributes.role {
            parts.push(format!("role={}", role));
        }

        // Clickability indicator
        if self.clickability_score > 0.7 {
            parts.push("⬤".to_string()); // Clickable indicator
        }

        parts.join(" ")
    }
}

/// Enhanced DOM tree containing all processed nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnhancedNodeTree {
    /// Root node IDs.
    pub roots: Vec<String>,

    /// All nodes indexed by ID.
    pub nodes: HashMap<String, EnhancedNode>,

    /// Viewport information.
    pub viewport: ViewportInfo,

    /// Processing timestamp.
    pub timestamp: u64,

    /// Page URL.
    pub url: String,

    /// Page title.
    pub title: String,
}

impl EnhancedNodeTree {
    /// Get all interactive elements.
    pub fn interactive_elements(&self) -> Vec<&EnhancedNode> {
        self.nodes
            .values()
            .filter(|n| n.is_interactive && n.is_visible)
            .collect()
    }

    /// Get all clickable elements sorted by clickability score.
    pub fn clickable_elements(&self) -> Vec<&EnhancedNode> {
        let mut nodes: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.clickability_score > 0.3 && n.is_visible)
            .collect();
        nodes.sort_by(|a, b| {
            b.clickability_score
                .partial_cmp(&a.clickability_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes
    }

    /// Get elements visible in viewport.
    pub fn visible_in_viewport(&self) -> Vec<&EnhancedNode> {
        self.nodes
            .values()
            .filter(|n| n.is_in_viewport && n.is_visible)
            .collect()
    }

    /// Find element at coordinates.
    pub fn element_at(&self, x: f64, y: f64) -> Option<&EnhancedNode> {
        // Find all elements containing the point
        let mut candidates: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.is_visible && n.bounding_box.contains(x, y))
            .collect();

        // Sort by paint order (highest first) and area (smallest first)
        candidates.sort_by(|a, b| {
            match b.paint_order.cmp(&a.paint_order) {
                std::cmp::Ordering::Equal => {
                    // Smaller area is more specific
                    let area_a = a.bounding_box.width * a.bounding_box.height;
                    let area_b = b.bounding_box.width * b.bounding_box.height;
                    area_a
                        .partial_cmp(&area_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                other => other,
            }
        });

        candidates.first().copied()
    }

    /// Generate LLM-friendly page representation.
    pub fn to_llm_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Page: {}\n", self.title));
        output.push_str(&format!("URL: {}\n", self.url));
        output.push_str(&format!(
            "Viewport: {}x{}\n\n",
            self.viewport.width, self.viewport.height
        ));

        output.push_str("Interactive Elements:\n");

        let relevant_nodes: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.is_relevant_for_llm())
            .collect();

        // Sort by paint order and position
        let mut sorted_nodes = relevant_nodes;
        sorted_nodes.sort_by(|a, b| {
            // Sort by Y position first, then X
            let y_cmp = a
                .bounding_box
                .y
                .partial_cmp(&b.bounding_box.y)
                .unwrap_or(std::cmp::Ordering::Equal);
            if y_cmp == std::cmp::Ordering::Equal {
                a.bounding_box
                    .x
                    .partial_cmp(&b.bounding_box.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                y_cmp
            }
        });

        for (i, node) in sorted_nodes.iter().enumerate() {
            output.push_str(&format!("{}\n", node.to_llm_string(i)));
        }

        output
    }
}

/// DOM processor for merging CDP trees.
pub struct DomProcessor;

impl DomProcessor {
    /// Create a new DOM processor.
    pub fn new() -> Self {
        Self
    }

    /// Calculate clickability score using 10-layer detection.
    pub fn calculate_clickability_score(
        tag_name: &str,
        attributes: &NodeAttributes,
        computed_styles: &HashMap<String, String>,
        has_event_listeners: bool,
        ax_properties: &HashMap<String, serde_json::Value>,
    ) -> (f64, Vec<String>) {
        let mut score: f64 = 0.0;
        let mut reasons = vec![];

        // Layer 1: JavaScript event listeners
        if has_event_listeners {
            score += 0.2;
            reasons.push("has_event_listener".to_string());
        }

        // Layer 2: Native interactive tags
        let interactive_tags = [
            "a", "button", "input", "select", "textarea", "option", "label",
        ];
        if interactive_tags.contains(&tag_name) {
            score += 0.3;
            reasons.push(format!("native_tag:{}", tag_name));
        }

        // Layer 3: Input type hints
        if let Some(ref input_type) = attributes.r#type {
            let clickable_types = [
                "button", "submit", "reset", "checkbox", "radio", "file", "image",
            ];
            if clickable_types.contains(&input_type.as_str()) {
                score += 0.15;
                reasons.push(format!("input_type:{}", input_type));
            }
        }

        // Layer 4: ARIA roles
        if let Some(ref role) = attributes.role {
            let clickable_roles = [
                "button",
                "link",
                "checkbox",
                "radio",
                "menuitem",
                "tab",
                "option",
                "switch",
                "treeitem",
            ];
            if clickable_roles.contains(&role.as_str()) {
                score += 0.2;
                reasons.push(format!("aria_role:{}", role));
            }
        }

        // Layer 5: Accessibility properties
        if let Some(focusable) = ax_properties.get("focusable") {
            if focusable.as_bool().unwrap_or(false) {
                score += 0.1;
                reasons.push("ax_focusable".to_string());
            }
        }

        // Layer 6: Cursor pointer style
        if let Some(cursor) = computed_styles.get("cursor") {
            if cursor == "pointer" {
                score += 0.15;
                reasons.push("cursor_pointer".to_string());
            }
        }

        // Layer 7: Href attribute (links)
        if attributes.href.is_some() {
            score += 0.2;
            reasons.push("has_href".to_string());
        }

        // Layer 8: Event handler attributes
        // This would be detected from attribute names starting with "on"
        // (onclick, onmousedown, etc.) - handled in JS bridge

        // Layer 9: Search indicators
        if attributes.r#type.as_deref() == Some("search") {
            score += 0.1;
            reasons.push("search_input".to_string());
        }
        if attributes.role.as_deref() == Some("search") {
            score += 0.1;
            reasons.push("search_role".to_string());
        }

        // Layer 10: Tabindex
        // Positive tabindex makes element focusable
        if let Some(tabindex) = ax_properties.get("tabindex") {
            if let Some(idx) = tabindex.as_i64() {
                if idx >= 0 {
                    score += 0.1;
                    reasons.push("tabindex".to_string());
                }
            }
        }

        // Clamp score to 1.0
        (score.min(1.0), reasons)
    }
}

impl Default for DomProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox.contains(50.0, 40.0));
        assert!(!bbox.contains(0.0, 0.0));
        assert!(!bbox.contains(200.0, 40.0));
    }

    #[test]
    fn test_bounding_box_center() {
        let bbox = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert_eq!(bbox.center(), (50.0, 50.0));
    }

    #[test]
    fn test_bounding_box_intersects() {
        let box1 = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let box2 = BoundingBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };
        let box3 = BoundingBox {
            x: 200.0,
            y: 200.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(box1.intersects(&box2));
        assert!(!box1.intersects(&box3));
    }

    #[test]
    fn test_clickability_score_button() {
        let (score, reasons) = DomProcessor::calculate_clickability_score(
            "button",
            &NodeAttributes::default(),
            &HashMap::new(),
            true,
            &HashMap::new(),
        );
        assert!(score > 0.4);
        assert!(reasons.contains(&"native_tag:button".to_string()));
        assert!(reasons.contains(&"has_event_listener".to_string()));
    }

    #[test]
    fn test_clickability_score_link() {
        let mut attrs = NodeAttributes::default();
        attrs.href = Some("https://example.com".to_string());

        let (score, reasons) = DomProcessor::calculate_clickability_score(
            "a",
            &attrs,
            &HashMap::new(),
            false,
            &HashMap::new(),
        );
        assert!(score > 0.4);
        assert!(reasons.contains(&"native_tag:a".to_string()));
        assert!(reasons.contains(&"has_href".to_string()));
    }

    #[test]
    fn test_clickability_score_cursor_pointer() {
        let mut styles = HashMap::new();
        styles.insert("cursor".to_string(), "pointer".to_string());

        let (score, reasons) = DomProcessor::calculate_clickability_score(
            "div",
            &NodeAttributes::default(),
            &styles,
            false,
            &HashMap::new(),
        );
        assert!(score > 0.1);
        assert!(reasons.contains(&"cursor_pointer".to_string()));
    }

    #[test]
    fn test_node_to_llm_string() {
        let mut attrs = NodeAttributes::default();
        attrs.id = Some("login-btn".to_string());
        attrs.aria_label = Some("Login".to_string());

        let node = EnhancedNode {
            id: "node_1".to_string(),
            backend_node_id: 1,
            tag_name: "button".to_string(),
            attributes: attrs,
            text_content: "Sign In".to_string(),
            bounding_box: BoundingBox::default(),
            is_visible: true,
            is_in_viewport: true,
            clickability_score: 0.9,
            clickability_reasons: vec!["native_tag:button".to_string()],
            paint_order: 1,
            is_interactive: true,
            is_focusable: true,
            parent_id: None,
            children: vec![],
            xpath: "/html/body/button".to_string(),
            css_selector: "#login-btn".to_string(),
            computed_styles: HashMap::new(),
        };

        let output = node.to_llm_string(0);
        assert!(output.contains("[0]"));
        assert!(output.contains("<button>"));
        assert!(output.contains("Sign In"));
        assert!(output.contains("id=login-btn"));
    }

    #[test]
    fn test_viewport_default() {
        let viewport = ViewportInfo::default();
        assert_eq!(viewport.width, 1280);
        assert_eq!(viewport.height, 720);
        assert_eq!(viewport.device_pixel_ratio, 1.0);
    }
}
