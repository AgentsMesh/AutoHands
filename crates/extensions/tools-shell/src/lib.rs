//! Shell execution tools for AutoHands.
//!
//! This extension provides:
//! - `exec`: Execute shell commands
//! - `shell_session`: Manage persistent shell sessions
//! - `background`: Manage background processes

mod background;
mod background_tool;
mod exec;
mod extension;
mod session;
mod session_tool;

pub use background::BackgroundManager;
pub use background_tool::BackgroundTool;
pub use exec::ExecTool;
pub use extension::ShellExtension;
pub use session::SessionManager;
pub use session_tool::SessionTool;
