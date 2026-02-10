//! DOM processing with Browser-Use style 3-tree merging.
//!
//! This module implements intelligent DOM analysis for LLM-based browser automation.
//! It merges information from multiple CDP trees to produce enhanced nodes with
//! accurate clickability detection.

mod dom_node;
mod dom_processor;
mod dom_tree;
mod dom_types;

pub use dom_node::EnhancedNode;
pub use dom_processor::DomProcessor;
pub use dom_tree::EnhancedNodeTree;
pub use dom_types::{NodeAttributes, ViewportInfo};

#[cfg(test)]
#[path = "dom_tests.rs"]
mod tests;
