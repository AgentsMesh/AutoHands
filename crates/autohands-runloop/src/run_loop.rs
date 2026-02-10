//! RunLoop core type definitions and construction.
//!
//! The RunLoop is the central event loop for the AutoHands framework,
//! inspired by iOS CFRunLoop design.

use std::collections::HashSet;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{mpsc, RwLock};

use autohands_core::registry::ChannelRegistry;

use crate::agent_driver::AgentEventHandler;
use crate::config::RunLoopConfig;
use crate::metrics::RunLoopMetrics;
use crate::mode::{RunLoopMode, RunLoopState};
use crate::observer::ObserverHandle;
use crate::source::{PortMessage, Source0, Source1Receiver};
use crate::spawner::SpawnerInner;
use crate::task_queue::TaskQueue;

// Additional impl blocks in separate files declared in lib.rs:
// - run_loop_accessors: state/metrics accessors & control methods
// - run_loop_execution: run() & run_in_mode() event loop
// - run_loop_processing: task processing & source management
// - run_loop_handlers: observer/wakeup handling
// - run_loop_traits: Default & TaskSubmitter impls

/// Wakeup signal for the RunLoop.
///
/// Similar to Mach Message in iOS.
#[derive(Debug, Clone)]
pub enum WakeupSignal {
    /// Source1 has a message ready.
    SourceReady {
        source_id: String,
        message: PortMessage,
    },
    /// Explicit wakeup request.
    Explicit { reason: String },
    /// Stop the RunLoop.
    Stop,
}

/// Mode-specific data.
pub(crate) struct ModeData {
    /// Source0 list (manually triggered).
    pub(crate) sources0: RwLock<Vec<Arc<dyn Source0>>>,
    /// Mode-specific observers.
    pub(crate) observers: RwLock<Vec<ObserverHandle>>,
}

impl ModeData {
    pub(crate) fn new() -> Self {
        Self {
            sources0: RwLock::new(Vec::new()),
            observers: RwLock::new(Vec::new()),
        }
    }
}

/// The RunLoop manages the event loop for AutoHands.
///
/// Inspired by iOS CFRunLoop, it provides:
/// - Mode isolation for different event types
/// - Source0 (manual) and Source1 (port) event sources
/// - Observer notifications at specific phases
/// - Sleep/wake mechanism for efficient CPU usage
pub struct RunLoop {
    /// Current mode.
    pub(crate) current_mode: RwLock<RunLoopMode>,
    /// Mode data (sources, observers per mode).
    pub(crate) modes: DashMap<RunLoopMode, ModeData>,
    /// Common modes set.
    pub(crate) common_modes: RwLock<HashSet<RunLoopMode>>,
    /// Current state.
    pub(crate) state: AtomicU8,
    /// Wakeup channel sender.
    pub(crate) wakeup_tx: mpsc::Sender<WakeupSignal>,
    /// Wakeup channel receiver.
    pub(crate) wakeup_rx: RwLock<mpsc::Receiver<WakeupSignal>>,
    /// Source1 receivers.
    pub(crate) source1_receivers: RwLock<Vec<Source1Receiver>>,
    /// Global observers (all modes).
    pub(crate) global_observers: RwLock<Vec<ObserverHandle>>,
    /// Task queue.
    pub(crate) task_queue: Arc<TaskQueue>,
    /// Configuration.
    pub(crate) _config: RunLoopConfig,
    /// Metrics.
    pub(crate) metrics: Arc<RunLoopMetrics>,
    /// Spawner inner state for task tracking.
    pub(crate) spawner_inner: Arc<SpawnerInner>,
    /// Agent event handler for processing tasks.
    pub(crate) handler: RwLock<Option<Arc<dyn AgentEventHandler>>>,
    /// Channel registry for sending responses.
    pub(crate) channel_registry: RwLock<Option<Arc<ChannelRegistry>>>,
}

impl RunLoop {
    /// Create a new RunLoop.
    pub fn new(config: RunLoopConfig) -> Self {
        let (wakeup_tx, wakeup_rx) = mpsc::channel(1024);

        let task_queue = Arc::new(TaskQueue::new(
            config.queue.clone(),
            config.chain.max_tasks_per_chain,
        ));

        let run_loop = Self {
            current_mode: RwLock::new(RunLoopMode::Default),
            modes: DashMap::new(),
            common_modes: RwLock::new(RunLoopMode::default_common_modes()),
            state: AtomicU8::new(RunLoopState::Created as u8),
            wakeup_tx,
            wakeup_rx: RwLock::new(wakeup_rx),
            source1_receivers: RwLock::new(Vec::new()),
            global_observers: RwLock::new(Vec::new()),
            task_queue,
            _config: config,
            metrics: Arc::new(RunLoopMetrics::new()),
            spawner_inner: Arc::new(SpawnerInner::new()),
            handler: RwLock::new(None),
            channel_registry: RwLock::new(None),
        };

        // Initialize default modes
        run_loop.modes.insert(RunLoopMode::Default, ModeData::new());
        run_loop
            .modes
            .insert(RunLoopMode::AgentProcessing, ModeData::new());
        run_loop
            .modes
            .insert(RunLoopMode::Background, ModeData::new());

        run_loop
    }
}

#[cfg(test)]
#[path = "run_loop_tests.rs"]
mod tests;
