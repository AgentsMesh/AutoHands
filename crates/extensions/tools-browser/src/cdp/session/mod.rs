//! CDP page session for interacting with a single page.

mod core;
mod dom;
mod input;
mod js;
mod navigation;

pub use self::core::PageSession;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
