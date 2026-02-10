//! # AutoHands RunLoop
//!
//! RunLoop-driven event loop for the AutoHands 24/7 autonomous agent framework.
//!
//! ## Design Inspiration
//!
//! The RunLoop architecture is inspired by iOS CFRunLoop, a battle-tested
//! event loop design with 20+ years of production experience:
//!
//! - **Mode isolation**: Different modes handle different event types
//! - **Source0/Source1 dual-track**: Manual vs. port-triggered sources
//! - **Observer phase notifications**: Entry → BeforeTimers → BeforeSources → BeforeWaiting → AfterWaiting → Exit
//! - **Sleep/wake mechanism**: Efficient CPU usage through proper sleep/wake cycles
//! - **Batch commit**: Similar to CATransaction, events are committed at BeforeWaiting
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                              Daemon Layer                                    │
//! │  (autohands-daemon) - PID management, signal handling, health checks        │
//! └────────────────────────────────────┬────────────────────────────────────────┘
//!                                      │
//! ┌────────────────────────────────────▼────────────────────────────────────────┐
//! │                           RunLoop (永续运行)                                 │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                         Mode Manager                                    │ │
//! │  │   DefaultMode ─────┬── Source0: Scheduler, AgentEvents                 │ │
//! │  │                    ├── Source1: WebSocket, Webhook, FileWatcher        │ │
//! │  │                    └── Observers: Checkpoint, Metrics, Cleanup         │ │
//! │  │   AgentProcessingMode ── Focus on Agent execution                      │ │
//! │  │   BackgroundMode ──────── Low-priority maintenance tasks               │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - [`RunLoop`]: The core event loop
//! - [`RunLoopMode`]: Mode isolation (Default, AgentProcessing, Background)
//! - [`RunLoopPhase`]: Execution phases (Entry, BeforeTimers, etc.)
//! - [`Source0`]: Manually triggered event sources
//! - [`Source1`]: Port-triggered event sources
//! - [`RunLoopObserver`]: Phase observers
//! - [`Task`]: Tasks flowing through the loop
//! - [`Timer`]: High-level timer abstraction
//! - [`AgentDriver`]: Agent execution integration
//!
//! ## Example
//!
//! ```rust,no_run
//! use autohands_runloop::{RunLoop, RunLoopConfig, RunLoopMode};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = RunLoopConfig::default();
//!     let run_loop = Arc::new(RunLoop::new(config));
//!
//!     // Run with a timeout
//!     let result = run_loop
//!         .run_in_mode(RunLoopMode::Default, Duration::from_secs(60))
//!         .await;
//! }
//! ```

pub mod agent_driver;
pub mod agent_source;
pub mod config;
pub mod correlation;
pub mod cron_timer;
pub mod error;
pub mod task;
pub mod task_chain;
pub mod task_queue;
pub mod integration;
pub mod metrics;
pub mod mode;
pub mod observer;
pub mod run_loop;
mod run_loop_accessors;
mod run_loop_execution;
mod run_loop_handlers;
mod run_loop_processing;
mod run_loop_task_dispatch;
mod run_loop_traits;
mod run_loop_wakeup;
pub mod source;
pub mod spawner;
pub mod spawner_types;
pub mod timer;

// Re-exports
pub use agent_driver::{AgentDriver, AgentEventHandler, AgentExecutionContext, AgentResult, ExecutionStatus};
pub use agent_source::{AgentTaskInjector, AgentSource0};
pub use config::{TaskChainConfig, TaskQueueConfig, RetryConfig, RunLoopConfig, WorkerPoolConfig};
pub use error::{TaskChainError, RunLoopError, RunLoopResult};
pub use task::{Task, TaskPriority, TaskSource};
pub use task_chain::TaskChainTracker;
pub use task_queue::TaskQueue;
pub use metrics::{MetricsSnapshot, RunLoopMetrics};
pub use mode::{RunLoopMode, RunLoopPhase, RunLoopRunResult, RunLoopState};
pub use observer::{
    EventBatchCommitObserver, LoggingObserver, MetricsObserver, ObserverHandle,
    ResourceCleanupObserver, RunLoopObserver, SpawnerObserver,
};
pub use run_loop::{RunLoop, WakeupSignal};
pub use source::{PortMessage, Source0, Source0Base, Source1, Source1Receiver};
pub use timer::{Timer, TimerBuilder};
pub use cron_timer::{CronTimer, CronTimerBuilder, schedules as cron_schedules};
pub use spawner::{
    CorrelationGuard, RunLoopSpawner, SpawnedTaskHandle, SpawnerInner, SpawnerMetrics,
    SpawnerStateProvider, TaskInfo, TaskState,
};
// Re-export CancellationToken for convenience
pub use tokio_util::sync::CancellationToken;

// Integration re-exports
pub use integration::checkpoint::{
    CheckpointError, CheckpointManager, CheckpointObserver, MemoryCheckpointManager,
    RunLoopCheckpoint,
};
// Note: TaskSubmitter is implemented directly on RunLoop
pub use integration::health::{
    HealthCheckError, HealthCheckObserver, HealthCheckable, HealthStatus,
    LivenessCheck, MemoryCheck, TaskQueueCheck,
};
pub use integration::scheduler::{JobInfo, SchedulerSource0, SchedulerTick};
pub use integration::signal::{SignalEvent, SignalSender, SignalSource1};
pub use integration::runtime::{RuntimeAgentEventHandler, RuntimeAgentEventHandlerBuilder};
pub use integration::websocket::{HttpTaskInjector, WebSocketSender, WebSocketSource1, WsMessageType};

// Trigger types (shared by file_watcher and webhook)
pub use integration::trigger_types::{
    FileWatcherConfig, Trigger, TriggerError, TriggerEvent, TriggersConfig, WebhookConfig,
};
// File watcher exports
pub use integration::file_watcher::FileWatcherTrigger;
pub use integration::file_watcher_manager::FileWatcherManager;
pub use integration::file_watcher_source::{FileChangeEvent, FileChangeType, FileWatcherSource1};
// Webhook exports
pub use integration::webhook::{WebhookEvent, WebhookSource1, WebhookTrigger};

// Channel bridge exports
pub use integration::channel_bridge::{ChannelBridge, ChannelBridgeConfig};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
