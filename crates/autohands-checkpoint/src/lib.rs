//! # AutoHands Checkpoint
//!
//! Checkpoint and recovery system for 24/7 autonomous agent framework.
//!
//! ## Features
//!
//! - Automatic checkpoint saving every N turns
//! - Full execution state serialization
//! - Recovery from latest checkpoint after crash

pub mod config;
pub mod error;
pub mod checkpoint;
pub mod recovery;
pub mod store;

pub use config::CheckpointConfig;
pub use error::CheckpointError;
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use recovery::RecoveryManager;
pub use store::{CheckpointStore, FileCheckpointStore, MemoryCheckpointStore};
