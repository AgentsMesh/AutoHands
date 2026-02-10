//! Linux Systemd service management.
//!
//! This module provides functionality to generate and manage Linux Systemd
//! service unit files for running AutoHands as a system service.

mod systemd_config;
mod systemd_ops;
mod systemd_service;

pub use systemd_config::SystemdConfig;
pub use systemd_ops::SystemdStatus;
pub use systemd_service::SystemdService;

#[cfg(test)]
#[path = "systemd_tests.rs"]
mod tests;
