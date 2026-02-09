//! Registries for extensions, tools, providers, memory backends, and channels.
//!
//! All registries use the `BaseRegistry<T>` pattern for consistent behavior:
//! - Thread-safe storage using DashMap
//! - Register/unregister with duplicate checking
//! - Get by ID, list all, iteration

mod base;
mod channel;
mod extension;
mod memory;
mod provider;
mod tool;

pub use base::{BaseRegistry, Registerable};
pub use channel::ChannelRegistry;
pub use extension::ExtensionRegistry;
pub use memory::MemoryRegistry;
pub use provider::ProviderRegistry;
pub use tool::ToolRegistry;
