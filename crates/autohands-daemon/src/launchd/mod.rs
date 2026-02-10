//! macOS LaunchAgent management.
//!
//! This module provides functionality to generate and manage macOS LaunchAgent
//! plist files for running AutoHands as a system service.

mod launchd_agent;
mod launchd_config;
mod launchd_ops;

pub use launchd_agent::LaunchAgent;
pub use launchd_config::LaunchAgentConfig;
pub use launchd_ops::LaunchAgentStatus;

#[cfg(test)]
#[path = "launchd_tests.rs"]
mod tests;
