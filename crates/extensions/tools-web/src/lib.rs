//! Web tools for AutoHands.
//!
//! Provides web_fetch and web_search tools.

mod extension;
mod tools;

pub use extension::WebToolsExtension;
pub use tools::{WebFetchTool, WebSearchTool};
