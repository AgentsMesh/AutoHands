//! # AutoHands Notify Tools Extension
//!
//! This extension provides tools for agents to send notifications
//! through various channels (email, Slack, Telegram, etc.).
//!
//! ## Tools
//!
//! - `notify_send`: Send a notification through a configured channel

pub mod extension;
pub mod tools;

pub use extension::NotifyToolsExtension;
