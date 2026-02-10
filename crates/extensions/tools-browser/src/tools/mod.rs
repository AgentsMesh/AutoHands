//! Browser automation tools.

mod content;
mod interaction;
mod navigation;
mod page;

pub use content::*;
pub use interaction::*;
pub use navigation::*;
pub use page::*;

// Shared default value helpers used by multiple submodules.

pub(crate) fn default_timeout() -> u64 {
    30000
}

pub(crate) fn default_content_type() -> String {
    "text".to_string()
}

pub(crate) fn default_compact() -> bool {
    true
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
