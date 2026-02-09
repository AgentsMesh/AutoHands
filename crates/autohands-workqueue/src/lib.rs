//! # AutoHands Queue
//!
//! Task queue system for 24/7 autonomous agent framework.
//!
//! ## Features
//!
//! - Priority queue
//! - Worker pool with concurrent execution
//! - Task state persistence (SQLite)
//! - Retry with dead letter queue
//! - Integration with Scheduler and AgentLoop

pub mod config;
pub mod error;
pub mod queue;
pub mod task;
pub mod worker;
pub mod store;

pub use config::QueueConfig;
pub use error::QueueError;
pub use queue::TaskQueue;
pub use task::{Task, TaskPriority, TaskStatus};
pub use worker::{Worker, WorkerPool};
pub use store::{FileTaskStore, MemoryTaskStore, TaskStore};
