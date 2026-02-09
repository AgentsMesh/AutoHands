//! # AutoHands Cron Tools Extension
//!
//! This extension provides tools for agents to manage scheduled tasks (cron jobs).
//! It allows agents to create, list, and delete their own scheduled tasks.
//!
//! ## Tools
//!
//! - `cron_create`: Create a new scheduled task
//! - `cron_list`: List all scheduled tasks
//! - `cron_delete`: Delete a scheduled task
//! - `cron_status`: Get status of a scheduled task

pub mod extension;
pub mod tools;

pub use extension::CronToolsExtension;
