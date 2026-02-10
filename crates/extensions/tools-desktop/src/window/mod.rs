//! Window management functionality.
//!
//! Provides cross-platform window control capabilities.

mod parsers;
mod window_controller;
mod window_controller_ops;
mod window_types;

pub use window_controller::WindowController;
pub use window_types::{WindowError, WindowInfo};

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
