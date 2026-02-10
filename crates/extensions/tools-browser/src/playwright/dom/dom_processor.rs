//! DomProcessor: 10-layer clickability detection.

use std::collections::HashMap;

use super::NodeAttributes;

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
                "button", "link", "checkbox", "radio", "menuitem",
                "tab", "option", "switch", "treeitem",
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

        // Layer 8: Event handler attributes (handled in JS bridge)

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
        if let Some(tabindex) = ax_properties.get("tabindex") {
            if let Some(idx) = tabindex.as_i64() {
                if idx >= 0 {
                    score += 0.1;
                    reasons.push("tabindex".to_string());
                }
            }
        }

        (score.min(1.0), reasons)
    }
}

impl Default for DomProcessor {
    fn default() -> Self {
        Self::new()
    }
}
