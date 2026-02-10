//! Browser instance manager.
//!
//! This module provides a unified browser manager interface using CDP.
//! It automatically launches Chrome with a persistent profile for login state preservation.

mod manager_core;
mod manager_pages;
mod manager_types;

pub use manager_core::BrowserManager;
pub use manager_types::{BrowserError, BrowserManagerConfig};

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
