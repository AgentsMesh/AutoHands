//! Markdown-based persistent memory backend for AutoHands.
//!
//! Stores memories as individual Markdown files with YAML front matter.
//! This format is human-readable, version-control friendly, and easy to edit.
//!
//! ## Storage Format
//!
//! Each memory is stored as a `.md` file:
//!
//! ```markdown
//! ---
//! id: mem_abc123
//! type: fact
//! tags:
//!   - project
//!   - meeting
//! importance: 0.8
//! created: 2024-02-07T10:30:00Z
//! updated: 2024-02-07T10:30:00Z
//! ---
//!
//! # Meeting Notes
//!
//! The actual memory content goes here...
//! ```

mod backend;
mod error;
mod extension;
mod parser;

pub use backend::MarkdownMemoryBackend;
pub use error::MarkdownMemoryError;
pub use extension::MarkdownMemoryExtension;
pub use parser::{MarkdownMemory, MarkdownParser};
