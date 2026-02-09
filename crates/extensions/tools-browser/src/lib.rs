//! Browser automation tools for AutoHands.
//!
//! Provides browser control via Chrome DevTools Protocol (CDP) with Browser-Use
//! style DOM analysis. Pure Rust implementation with zero Node.js dependencies.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    WebSocket     ┌──────────────────┐
//! │  Rust Backend   │ ◄──────────────► │   Chrome/Edge    │
//! │  (this crate)   │       CDP        │  (user's browser)│
//! └─────────────────┘                  └──────────────────┘
//! ```
//!
//! ## Setup
//!
//! Start Chrome with remote debugging enabled:
//!
//! ```bash
//! # macOS
//! /Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222
//!
//! # Linux
//! google-chrome --remote-debugging-port=9222
//!
//! # Windows
//! chrome.exe --remote-debugging-port=9222
//! ```
//!
//! This allows AutoHands to use your existing browser sessions with all logins preserved.
//!
//! ## Lazy Initialization
//!
//! The browser is NOT connected when the extension is loaded. It is lazily
//! initialized when `browser_open` is first called, ensuring fast startup.
//!
//! ## Tools
//!
//! ### Basic Tools
//! - `browser_open` - Open a new page (triggers lazy browser connection)
//! - `browser_navigate` - Navigate to a URL
//! - `browser_click` - Click an element
//! - `browser_type` - Type text into an input
//! - `browser_screenshot` - Take a screenshot
//! - `browser_get_content` - Get page/element content
//! - `browser_execute_js` - Execute JavaScript
//! - `browser_wait_for` - Wait for an element
//!
//! ### AI-Powered Tools (requires vision-capable LLM)
//! - `browser_ai_click` - Click an element by natural language description
//! - `browser_ai_fill` - Fill a form field by natural language description
//! - `browser_ai_extract` - Extract structured data from page using AI
//!
//! ### DOM Analysis (Browser-Use Style)
//! - `browser_get_dom` - Get enhanced DOM tree with clickability scores
//!
//! ## DOM Processing
//!
//! Inspired by Browser-Use, we provide intelligent DOM analysis with:
//! - 10-layer clickable detection (event listeners, ARIA roles, cursor, etc.)
//! - LLM-friendly element serialization
//! - Accurate bounding boxes for coordinate-based clicks

mod ai_tools;
pub mod cdp;
mod dom;
mod extension;
pub mod manager;
mod tools;

pub use ai_tools::{AiClickTool, AiExtractTool, AiFillTool, VisionProvider};
pub use cdp::{CdpClient, CdpError, PageSession};
pub use dom::{DomProcessor, EnhancedNode, EnhancedNodeTree, NodeAttributes, ViewportInfo};
pub use extension::BrowserToolsExtension;
pub use manager::{BrowserError, BrowserManager, BrowserManagerConfig};
pub use tools::*;
