//! WebSocket interface module.
//!
//! Provides real-time bidirectional communication with clients.
//! All WebSocket messages are converted to RunLoop events for unified processing.
//!
//! ## Channel Integration
//!
//! `ApiWsChannel` implements the `Channel` trait, enabling the RunLoop to route
//! responses back to specific WebSocket connections via the ChannelRegistry.

mod channel;
mod connection;
mod handler;
mod message;

pub use channel::ApiWsChannel;
pub use connection::WsConnectionManager;
pub use handler::{ws_handler, ws_handler_with_runloop};
pub use message::WsMessage;
