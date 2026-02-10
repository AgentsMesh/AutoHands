//! Enhanced DOM node with merged information from multiple CDP trees.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::dom_types::{BoundingBox, NodeAttributes};

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

    /// CSS selector for this element (best effort unique selector).
    pub css_selector: String,

    /// Computed styles relevant for interaction.
    #[serde(default)]
    pub computed_styles: HashMap<String, String>,
}

impl EnhancedNode {
    /// Check if this node should be included in LLM output.
    pub fn is_relevant_for_llm(&self) -> bool {
        if !self.is_visible {
            return false;
        }

        if !self.is_in_viewport && !self.is_interactive {
            return false;
        }

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

        parts.push(format!("[{}]", index));

        let type_str = if let Some(ref t) = self.attributes.r#type {
            format!("<{} type={}>", self.tag_name, t)
        } else {
            format!("<{}>", self.tag_name)
        };
        parts.push(type_str);

        if !self.text_content.is_empty() {
            let text = if self.text_content.len() > 50 {
                format!("{}...", &self.text_content[..47])
            } else {
                self.text_content.clone()
            };
            parts.push(format!("\"{}\"", text.replace('\n', " ").trim()));
        }

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

        if self.clickability_score > 0.7 {
            parts.push("\u{2b24}".to_string()); // Clickable indicator
        }

        parts.join(" ")
    }
}
