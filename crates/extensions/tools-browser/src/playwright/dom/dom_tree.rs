//! EnhancedNodeTree: tree operations, querying, LLM output.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{EnhancedNode, ViewportInfo};

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
        let mut candidates: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.is_visible && n.bounding_box.contains(x, y))
            .collect();

        candidates.sort_by(|a, b| {
            match b.paint_order.cmp(&a.paint_order) {
                std::cmp::Ordering::Equal => {
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

        let mut sorted_nodes = relevant_nodes;
        sorted_nodes.sort_by(|a, b| {
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
