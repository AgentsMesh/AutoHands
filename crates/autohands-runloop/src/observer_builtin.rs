//! Built-in observer implementations for RunLoop.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::mode::RunLoopPhase;
use crate::RunLoop;

use super::RunLoopObserver;

/// Metrics collection observer.
///
/// Collects RunLoop metrics at BeforeWaiting phase.
pub struct MetricsObserver {
    activities: u32,
}

impl MetricsObserver {
    /// Create a new metrics observer.
    pub fn new() -> Self {
        Self {
            activities: RunLoopPhase::BeforeWaiting as u32 | RunLoopPhase::AfterWaiting as u32,
        }
    }

    /// Create with custom activities.
    pub fn with_activities(activities: u32) -> Self {
        Self { activities }
    }
}

impl Default for MetricsObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RunLoopObserver for MetricsObserver {
    fn activities(&self) -> u32 {
        self.activities
    }

    fn priority(&self) -> i32 {
        -100 // Run late, after other observers
    }

    async fn on_phase(&self, phase: RunLoopPhase, run_loop: &RunLoop) {
        let metrics = run_loop.metrics();

        match phase {
            RunLoopPhase::BeforeWaiting => {
                metrics.set_pending_events(run_loop.pending_task_count().await as u64);
            }
            RunLoopPhase::AfterWaiting => {
                metrics.record_wakeup();
            }
            _ => {}
        }
    }
}

/// Resource cleanup observer.
///
/// Cleans up resources at BeforeWaiting and Exit phases.
/// Similar to AutoreleasePool behavior in iOS.
pub struct ResourceCleanupObserver {
    cleanup_fn: Arc<dyn Fn() + Send + Sync>,
}

impl ResourceCleanupObserver {
    /// Create a new cleanup observer with a custom cleanup function.
    pub fn new<F>(cleanup_fn: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            cleanup_fn: Arc::new(cleanup_fn),
        }
    }
}

#[async_trait]
impl RunLoopObserver for ResourceCleanupObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32 | RunLoopPhase::Exit as u32
    }

    fn priority(&self) -> i32 {
        100 // Run early, before other observers
    }

    async fn on_phase(&self, phase: RunLoopPhase, _run_loop: &RunLoop) {
        debug!("Resource cleanup at phase: {:?}", phase);
        (self.cleanup_fn)();
    }
}

/// Event batch commit observer.
///
/// Commits batched events at BeforeWaiting phase.
/// Similar to CATransaction commit in iOS.
pub struct EventBatchCommitObserver {
    commit_fn: Arc<dyn Fn() + Send + Sync>,
}

impl EventBatchCommitObserver {
    /// Create a new batch commit observer.
    pub fn new<F>(commit_fn: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            commit_fn: Arc::new(commit_fn),
        }
    }
}

#[async_trait]
impl RunLoopObserver for EventBatchCommitObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32
    }

    fn priority(&self) -> i32 {
        0
    }

    async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {
        debug!("Committing batched events");
        (self.commit_fn)();
    }
}

/// Logging observer for debugging.
///
/// Logs all phase transitions.
pub struct LoggingObserver {
    name: String,
}

impl LoggingObserver {
    /// Create a new logging observer.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl RunLoopObserver for LoggingObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::ALL
    }

    fn priority(&self) -> i32 {
        -1000 // Run last
    }

    async fn on_phase(&self, phase: RunLoopPhase, _run_loop: &RunLoop) {
        debug!("[{}] RunLoop phase: {:?}", self.name, phase);
    }
}

/// One-shot observer that fires once then removes itself.
pub struct OneShotObserver<F>
where
    F: Fn(&RunLoop) + Send + Sync,
{
    phase: RunLoopPhase,
    callback: F,
}

impl<F> OneShotObserver<F>
where
    F: Fn(&RunLoop) + Send + Sync,
{
    /// Create a new one-shot observer.
    pub fn new(phase: RunLoopPhase, callback: F) -> Self {
        Self { phase, callback }
    }
}

#[async_trait]
impl<F> RunLoopObserver for OneShotObserver<F>
where
    F: Fn(&RunLoop) + Send + Sync,
{
    fn activities(&self) -> u32 {
        self.phase as u32
    }

    fn repeats(&self) -> bool {
        false
    }

    async fn on_phase(&self, _phase: RunLoopPhase, run_loop: &RunLoop) {
        (self.callback)(run_loop);
    }
}
