//! SQLite memory backend for AutoHands.
//!
//! Provides persistent memory storage using SQLite.

mod backend;
mod extension;
mod schema;

pub use backend::SqliteMemoryBackend;
pub use extension::SqliteMemoryExtension;
