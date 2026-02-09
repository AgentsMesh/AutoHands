//! Chrome DevTools Protocol (CDP) client implementation.
//!
//! This module provides a pure Rust CDP client for browser automation.
//! It connects to Chrome/Chromium via WebSocket and communicates using
//! the CDP JSON-RPC protocol.
//!
//! ## Usage
//!
//! 1. Start Chrome with remote debugging:
//!    ```bash
//!    chrome --remote-debugging-port=9222
//!    ```
//!
//! 2. Connect and automate:
//!    ```rust,ignore
//!    let client = CdpClient::connect("http://localhost:9222").await?;
//!    let page = client.new_page().await?;
//!    page.navigate("https://example.com").await?;
//!    ```

mod client;
mod error;
mod protocol;
mod session;

pub use client::CdpClient;
pub use error::CdpError;
pub use protocol::*;
pub use session::PageSession;
