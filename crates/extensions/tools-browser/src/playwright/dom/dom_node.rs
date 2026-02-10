//! BoundingBox geometry methods and EnhancedNode LLM methods.

use super::{BoundingBox, EnhancedNode, ViewportInfo};

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
