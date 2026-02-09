//! Integration adapters for existing AutoHands components.
//!
//! This module provides adapters to integrate existing components
//! (Scheduler, FileWatcher, Webhook, etc.) with the RunLoop architecture.
//!
//! Note: TaskSubmitter is now implemented directly on RunLoop,
//! no separate adapter needed.

pub mod checkpoint;
pub mod file_watcher;
pub mod health;
pub mod runtime;
pub mod scheduler;
pub mod signal;
pub mod webhook;
pub mod websocket;
