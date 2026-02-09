//! WebSocket interface module.
//!
//! Provides real-time bidirectional communication with clients.
//! All WebSocket messages are converted to RunLoop events for unified processing.

mod connection;
mod handler;
mod message;

pub use connection::WsConnectionManager;
pub use handler::{ws_handler, ws_handler_with_runloop};
pub use message::WsMessage;
