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

mod dom_node;
mod dom_processor;
mod dom_tree;
mod dom_types;

pub use dom_processor::DomProcessor;
pub use dom_tree::EnhancedNodeTree;
pub use dom_types::{BoundingBox, EnhancedNode, NodeAttributes, ViewportInfo};

#[cfg(test)]
#[path = "dom_tests.rs"]
mod tests;
