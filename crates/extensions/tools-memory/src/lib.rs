//! # AutoHands Memory Tools Extension
//!
//! Provides `memory_search`, `memory_get`, and `memory_store` tools
//! that allow agents to interact with long-term memory during conversations.

pub mod extension;
pub mod tools;

pub use extension::MemoryToolsExtension;
pub use tools::{MemoryGetTool, MemorySearchTool, MemoryStoreTool};
