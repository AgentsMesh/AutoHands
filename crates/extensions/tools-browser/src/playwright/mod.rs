//! Playwright-based browser automation backend.
//!
//! This module provides browser control using Playwright via a Node.js bridge.
//! The design is inspired by Browser-Use's architecture with 3-tree DOM merging
//! and intelligent clickable detection.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    JSON-RPC    ┌──────────────────┐
//! │  Rust Backend   │ ◄────────────► │ Node.js Bridge   │
//! │  (this module)  │                │ (playwright.js)  │
//! └─────────────────┘                └──────────────────┘
//!                                           │
//!                                    Playwright API
//!                                           │
//!                                    ┌──────────────────┐
//!                                    │    Browser       │
//!                                    │ (Chrome/Firefox) │
//!                                    └──────────────────┘
//! ```
//!
//! ## DOM Processing (Browser-Use Style)
//!
//! We merge 3 CDP trees for complete DOM understanding:
//! 1. **DOM Tree** - Full DOM structure
//! 2. **Accessibility Tree** - AX properties for interactive elements
//! 3. **DOMSnapshot** - Layout and computed styles
//!
//! The merged result produces `EnhancedNode` with:
//! - Accurate bounding boxes
//! - Clickability scores (10-layer detection)
//! - Paint order for z-index handling
//! - LLM-friendly serialization

mod bridge;
mod bridge_script;
mod browser_api;
mod dom;
mod error;
mod manager;

pub use bridge::{PlaywrightBridge, PlaywrightBridgeConfig};
pub use dom::{DomProcessor, EnhancedNode, EnhancedNodeTree, NodeAttributes, ViewportInfo};
pub use error::PlaywrightError;
pub use manager::{PlaywrightManager, PlaywrightManagerConfig};
