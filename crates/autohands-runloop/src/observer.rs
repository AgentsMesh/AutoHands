//! RunLoop observer definitions.
//!
//! Observers are notified at specific phases of the RunLoop,
//! similar to CFRunLoopObserver in iOS.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use std::time::Duration;

use crate::mode::RunLoopPhase;
use crate::spawner::SpawnerInner;
use crate::RunLoop;

/// RunLoop observer trait.
///
/// Similar to CFRunLoopObserver in iOS.
/// Observers are triggered at specific phases of the RunLoop.
#[async_trait]
pub trait RunLoopObserver: Send + Sync {
    /// Get the activity mask (which phases to observe).
    /// Use RunLoopPhase::ALL to observe all phases.
    fn activities(&self) -> u32;

    /// Whether the observer repeats (false = triggered once then removed).
    fn repeats(&self) -> bool {
        true
    }

    /// Observer priority (lower = executed first).
    fn priority(&self) -> i32 {
        0
    }

    /// Called when the observed phase is triggered.
    async fn on_phase(&self, phase: RunLoopPhase, run_loop: &RunLoop);
}

/// Observer registration handle.
pub struct ObserverHandle {
    id: String,
    observer: Arc<dyn RunLoopObserver>,
    fired: AtomicBool,
}

impl ObserverHandle {
    /// Create a new observer handle.
    pub fn new(id: impl Into<String>, observer: Arc<dyn RunLoopObserver>) -> Self {
        Self {
            id: id.into(),
            observer,
            fired: AtomicBool::new(false),
        }
    }

    /// Get the observer ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the observer.
    pub fn observer(&self) -> &Arc<dyn RunLoopObserver> {
        &self.observer
    }

    /// Check if this observer should be triggered for the given phase.
    pub fn should_trigger(&self, phase: RunLoopPhase) -> bool {
        if self.fired.load(Ordering::SeqCst) && !self.observer.repeats() {
            return false;
        }
        phase.matches(self.observer.activities())
    }

    /// Mark as fired.
    pub fn mark_fired(&self) {
        self.fired.store(true, Ordering::SeqCst);
    }

    /// Check if should be removed (fired and non-repeating).
    pub fn should_remove(&self) -> bool {
        self.fired.load(Ordering::SeqCst) && !self.observer.repeats()
    }
}

// ============================================================================
// Built-in Observer Implementations
// ============================================================================

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

/// Spawner lifecycle observer.
///
/// Manages spawned task lifecycle in coordination with RunLoop:
/// - BeforeWaiting: Log active task count, check for stale tasks
/// - Exit: Cancel all active tasks for graceful shutdown
pub struct SpawnerObserver {
    /// Shared spawner state.
    spawner_inner: Arc<SpawnerInner>,
    /// Task timeout duration (tasks running longer than this are considered stale).
    task_timeout: Option<Duration>,
    /// Whether to cancel tasks on Exit.
    cancel_on_exit: bool,
}

impl SpawnerObserver {
    /// Create a new spawner observer.
    pub fn new(spawner_inner: Arc<SpawnerInner>) -> Self {
        Self {
            spawner_inner,
            task_timeout: None,
            cancel_on_exit: true,
        }
    }

    /// Set task timeout for stale task detection.
    pub fn with_task_timeout(mut self, timeout: Duration) -> Self {
        self.task_timeout = Some(timeout);
        self
    }

    /// Configure whether to cancel tasks on Exit phase.
    pub fn with_cancel_on_exit(mut self, cancel: bool) -> Self {
        self.cancel_on_exit = cancel;
        self
    }

    /// Check for stale tasks (running longer than timeout).
    fn check_stale_tasks(&self) {
        if let Some(timeout) = self.task_timeout {
            let now = chrono::Utc::now();
            let mut stale_count = 0;

            for entry in self.spawner_inner.tasks.iter() {
                let task = entry.value();
                let duration = now - task.spawned_at;
                if duration > chrono::Duration::from_std(timeout).unwrap_or(chrono::Duration::MAX) {
                    tracing::warn!(
                        task_id = %task.id,
                        task_name = %task.name,
                        duration_secs = duration.num_seconds(),
                        "Stale task detected"
                    );
                    stale_count += 1;
                }
            }

            if stale_count > 0 {
                tracing::warn!(stale_count, "Found stale tasks");
            }
        }
    }

    /// Cancel all active tasks.
    ///
    /// Uses the SpawnerInner's cancel_all method which properly triggers
    /// CancellationTokens for cancellable tasks.
    fn cancel_all_tasks(&self) {
        let cancellable_count = self.spawner_inner.cancellable_count();
        let total_count = self.spawner_inner.tasks.len();

        if total_count > 0 {
            tracing::info!(
                total_count,
                cancellable_count,
                "Cancelling all active tasks on RunLoop exit"
            );

            // Cancel all cancellable tasks (triggers their CancellationTokens)
            let cancelled = self.spawner_inner.cancel_all();

            // Mark remaining non-cancellable tasks as cancelled
            let remaining_ids: Vec<uuid::Uuid> = self
                .spawner_inner
                .tasks
                .iter()
                .map(|e| *e.key())
                .collect();

            for id in remaining_ids {
                if let Some(mut entry) = self.spawner_inner.tasks.get_mut(&id) {
                    entry.state = crate::spawner::TaskState::Cancelled;
                }
                self.spawner_inner
                    .total_cancelled
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.spawner_inner.tasks.remove(&id);
            }

            tracing::info!(cancelled, "Cancelled cancellable tasks with tokens");
        }
    }
}

#[async_trait]
impl RunLoopObserver for SpawnerObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32 | RunLoopPhase::Exit as u32
    }

    fn priority(&self) -> i32 {
        50 // Run relatively early
    }

    async fn on_phase(&self, phase: RunLoopPhase, run_loop: &RunLoop) {
        match phase {
            RunLoopPhase::BeforeWaiting => {
                let active_tasks = self.spawner_inner.tasks.len();
                if active_tasks > 0 {
                    debug!(
                        active_tasks,
                        "Spawner status at BeforeWaiting"
                    );
                }

                // Check for stale tasks
                self.check_stale_tasks();

                // Update metrics
                let metrics = run_loop.metrics();
                metrics.set_active_tasks(active_tasks as u64);
            }
            RunLoopPhase::Exit => {
                if self.cancel_on_exit {
                    self.cancel_all_tasks();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn test_observer_handle() {
        struct TestObserver;

        #[async_trait]
        impl RunLoopObserver for TestObserver {
            fn activities(&self) -> u32 {
                RunLoopPhase::Entry as u32
            }

            async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {}
        }

        let handle = ObserverHandle::new("test", Arc::new(TestObserver));

        assert!(handle.should_trigger(RunLoopPhase::Entry));
        assert!(!handle.should_trigger(RunLoopPhase::Exit));
        assert!(!handle.should_remove());

        handle.mark_fired();
        assert!(!handle.should_remove()); // Still repeats by default
    }

    #[test]
    fn test_non_repeating_observer() {
        struct NonRepeatingObserver;

        #[async_trait]
        impl RunLoopObserver for NonRepeatingObserver {
            fn activities(&self) -> u32 {
                RunLoopPhase::Entry as u32
            }

            fn repeats(&self) -> bool {
                false
            }

            async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {}
        }

        let handle = ObserverHandle::new("test", Arc::new(NonRepeatingObserver));

        assert!(handle.should_trigger(RunLoopPhase::Entry));
        handle.mark_fired();
        assert!(!handle.should_trigger(RunLoopPhase::Entry));
        assert!(handle.should_remove());
    }

    #[test]
    fn test_metrics_observer() {
        let observer = MetricsObserver::new();

        assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
        assert!(RunLoopPhase::AfterWaiting.matches(observer.activities()));
        assert!(!RunLoopPhase::Entry.matches(observer.activities()));
    }

    #[test]
    fn test_resource_cleanup_observer() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let observer = ResourceCleanupObserver::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
        assert!(RunLoopPhase::Exit.matches(observer.activities()));
    }

    #[test]
    fn test_logging_observer() {
        let observer = LoggingObserver::new("test");

        assert_eq!(observer.activities(), RunLoopPhase::ALL);
        assert_eq!(observer.priority(), -1000);
    }

    #[test]
    fn test_one_shot_observer() {
        let observer = OneShotObserver::new(RunLoopPhase::Entry, |_| {});

        assert!(!observer.repeats());
        assert_eq!(observer.activities(), RunLoopPhase::Entry as u32);
    }

    #[test]
    fn test_spawner_observer_creation() {
        use crate::spawner::SpawnerInner;

        let inner = Arc::new(SpawnerInner::new());
        let observer = SpawnerObserver::new(inner);

        assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
        assert!(RunLoopPhase::Exit.matches(observer.activities()));
        assert_eq!(observer.priority(), 50);
    }

    #[test]
    fn test_spawner_observer_with_timeout() {
        use crate::spawner::SpawnerInner;

        let inner = Arc::new(SpawnerInner::new());
        let observer = SpawnerObserver::new(inner)
            .with_task_timeout(Duration::from_secs(60))
            .with_cancel_on_exit(false);

        assert!(observer.task_timeout.is_some());
        assert!(!observer.cancel_on_exit);
    }
}
