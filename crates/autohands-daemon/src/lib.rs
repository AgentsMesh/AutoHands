//! # AutoHands Daemon
//!
//! Daemon process management for 24/7 autonomous agent framework.
//!
//! ## Features
//!
//! - PID file management (prevents duplicate instances)
//! - Signal handling (SIGTERM/SIGINT for graceful shutdown, SIGHUP for config reload)
//! - Process daemonization (Unix fork)
//! - Health check loop
//! - Auto-restart on crash
//! - macOS LaunchAgent integration
//! - Linux Systemd integration
//!
//! ## Usage
//!
//! ```rust,ignore
//! use autohands_daemon::{Daemon, DaemonConfig};
//!
//! let config = DaemonConfig::default();
//! let daemon = Daemon::new(config)?;
//! daemon.start().await?;
//! ```
//!
//! ## System Service Installation
//!
//! ### macOS (LaunchAgent)
//!
//! ```rust,ignore
//! use autohands_daemon::launchd::{LaunchAgent, LaunchAgentConfig};
//!
//! let config = LaunchAgentConfig::default();
//! let agent = LaunchAgent::new(config);
//! agent.install()?;
//! ```
//!
//! ### Linux (Systemd)
//!
//! ```rust,ignore
//! use autohands_daemon::systemd::{SystemdService, SystemdConfig};
//!
//! let config = SystemdConfig::default();
//! let service = SystemdService::new(config);
//! service.install()?;
//! ```

pub mod config;
pub mod daemon;
pub mod error;
pub mod health;
pub mod pid;
pub mod runloop;
pub mod signal;

// Platform-specific modules
#[cfg(target_os = "macos")]
pub mod launchd;

#[cfg(target_os = "linux")]
pub mod systemd;

// Re-exports
pub use config::DaemonConfig;
pub use daemon::{Daemon, DaemonState};
pub use error::DaemonError;
pub use health::HealthChecker;
pub use pid::PidFile;
pub use runloop::{RunLoopDaemonBuilder, RunLoopRunner};

#[cfg(target_os = "macos")]
pub use launchd::{LaunchAgent, LaunchAgentConfig, LaunchAgentStatus};

#[cfg(target_os = "linux")]
pub use systemd::{SystemdConfig, SystemdService, SystemdStatus};
