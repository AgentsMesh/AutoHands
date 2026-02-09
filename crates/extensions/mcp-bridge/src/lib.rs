//! MCP (Model Context Protocol) bridge for AutoHands.
//!
//! Implements the MCP protocol for connecting to external MCP servers.
//! Supports multiple transports: stdio, HTTP, and SSE.

mod client;
mod extension;
mod http_transport;
mod protocol;
mod sse_transport;
mod transport;

pub use client::McpClient;
pub use extension::McpBridgeExtension;
pub use http_transport::{HttpTransport, HttpTransportConfig};
pub use protocol::{McpMethod, McpRequest, McpResponse};
pub use sse_transport::{SseTransport, SseTransportConfig};
pub use transport::{StdioTransport, Transport, TransportError};
