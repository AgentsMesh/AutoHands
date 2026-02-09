//! RunLoop core implementation.
//!
//! The RunLoop is the central event loop for the AutoHands framework,
//! inspired by iOS CFRunLoop design.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::config::RunLoopConfig;
use crate::error::{RunLoopError, RunLoopResult};
use crate::task::{Task, TaskQueue};
use crate::metrics::RunLoopMetrics;
use crate::mode::{RunLoopMode, RunLoopPhase, RunLoopRunResult, RunLoopState};
use crate::observer::{ObserverHandle, RunLoopObserver};
use crate::source::{PortMessage, Source0, Source1Receiver};
use crate::spawner::SpawnerInner;

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
struct ModeData {
    /// Source0 list (manually triggered).
    sources0: RwLock<Vec<Arc<dyn Source0>>>,

    /// Mode-specific observers.
    observers: RwLock<Vec<ObserverHandle>>,
}

impl ModeData {
    fn new() -> Self {
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
    current_mode: RwLock<RunLoopMode>,

    /// Mode data (sources, observers per mode).
    modes: DashMap<RunLoopMode, ModeData>,

    /// Common modes set.
    common_modes: RwLock<HashSet<RunLoopMode>>,

    /// Current state.
    state: AtomicU8,

    /// Wakeup channel sender.
    wakeup_tx: mpsc::Sender<WakeupSignal>,

    /// Wakeup channel receiver.
    wakeup_rx: RwLock<mpsc::Receiver<WakeupSignal>>,

    /// Source1 receivers.
    source1_receivers: RwLock<Vec<Source1Receiver>>,

    /// Global observers (all modes).
    global_observers: RwLock<Vec<ObserverHandle>>,

    /// Task queue.
    task_queue: Arc<TaskQueue>,

    /// Configuration.
    config: RunLoopConfig,

    /// Metrics.
    metrics: Arc<RunLoopMetrics>,

    /// Spawner inner state for task tracking.
    spawner_inner: Arc<SpawnerInner>,
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
            config,
            metrics: Arc::new(RunLoopMetrics::new()),
            spawner_inner: Arc::new(SpawnerInner::new()),
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

    /// Get current state.
    pub fn state(&self) -> RunLoopState {
        RunLoopState::from(self.state.load(Ordering::SeqCst))
    }

    /// Set state.
    fn set_state(&self, state: RunLoopState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }

    /// Get current mode.
    pub async fn current_mode(&self) -> RunLoopMode {
        self.current_mode.read().await.clone()
    }

    /// Get metrics.
    pub fn metrics(&self) -> &Arc<RunLoopMetrics> {
        &self.metrics
    }

    /// Get pending task count.
    pub async fn pending_task_count(&self) -> usize {
        self.task_queue.len().await
    }

    /// Get wakeup sender (for external wakeup).
    pub fn wakeup_sender(&self) -> mpsc::Sender<WakeupSignal> {
        self.wakeup_tx.clone()
    }

    /// Get task queue (for external task injection).
    ///
    /// This allows external components (like HTTP handlers) to inject
    /// tasks directly into the RunLoop's task queue.
    pub fn task_queue(&self) -> Arc<TaskQueue> {
        self.task_queue.clone()
    }

    /// Get the spawner inner state for metrics/monitoring.
    pub fn spawner_inner(&self) -> Arc<SpawnerInner> {
        self.spawner_inner.clone()
    }

    /// Get spawner task metrics.
    pub fn spawner_metrics(&self) -> crate::spawner::SpawnerMetrics {
        crate::spawner::SpawnerMetrics {
            total_spawned: self.spawner_inner.total_spawned.load(Ordering::SeqCst),
            total_completed: self.spawner_inner.total_completed.load(Ordering::SeqCst),
            total_cancelled: self.spawner_inner.total_cancelled.load(Ordering::SeqCst),
            total_failed: self.spawner_inner.total_failed.load(Ordering::SeqCst),
            active_tasks: self.spawner_inner.tasks.len(),
        }
    }

    // ========================================================================
    // Source Management
    // ========================================================================

    /// Add a Source0 to the specified modes.
    pub async fn add_source0(&self, source: Arc<dyn Source0>) {
        for mode in source.modes() {
            if *mode == RunLoopMode::Common {
                // Add to all common modes
                let common = self.common_modes.read().await;
                for m in common.iter() {
                    if let Some(mode_data) = self.modes.get(m) {
                        mode_data.sources0.write().await.push(source.clone());
                    }
                }
            } else if let Some(mode_data) = self.modes.get(mode) {
                mode_data.sources0.write().await.push(source.clone());
            }
        }
    }

    /// Add a Source1 receiver.
    pub async fn add_source1(&self, receiver: Source1Receiver) {
        self.source1_receivers.write().await.push(receiver);
    }

    /// Remove a Source0 by ID.
    pub async fn remove_source0(&self, source_id: &str) {
        for mode_data in self.modes.iter() {
            mode_data
                .sources0
                .write()
                .await
                .retain(|s| s.id() != source_id);
        }
    }

    // ========================================================================
    // Observer Management
    // ========================================================================

    /// Add a global observer (notified in all modes).
    pub async fn add_observer(&self, id: impl Into<String>, observer: Arc<dyn RunLoopObserver>) {
        let handle = ObserverHandle::new(id, observer);
        self.global_observers.write().await.push(handle);
        // Sort by priority
        self.global_observers
            .write()
            .await
            .sort_by_key(|h| h.observer().priority());
    }

    /// Add an observer to a specific mode.
    pub async fn add_mode_observer(
        &self,
        mode: &RunLoopMode,
        id: impl Into<String>,
        observer: Arc<dyn RunLoopObserver>,
    ) {
        if let Some(mode_data) = self.modes.get(mode) {
            let handle = ObserverHandle::new(id, observer);
            mode_data.observers.write().await.push(handle);
            mode_data
                .observers
                .write()
                .await
                .sort_by_key(|h| h.observer().priority());
        }
    }

    /// Remove an observer by ID.
    pub async fn remove_observer(&self, id: &str) {
        self.global_observers
            .write()
            .await
            .retain(|h| h.id() != id);
        for mode_data in self.modes.iter() {
            mode_data
                .observers
                .write()
                .await
                .retain(|h| h.id() != id);
        }
    }

    // ========================================================================
    // Task Management
    // ========================================================================

    /// Inject a task into the queue.
    pub async fn inject_task(&self, task: Task) -> RunLoopResult<()> {
        self.task_queue.enqueue(task).await?;
        self.metrics.record_event_enqueued();
        Ok(())
    }

    /// Wakeup the RunLoop.
    ///
    /// Similar to CFRunLoopWakeUp.
    pub fn wakeup(&self, reason: impl Into<String>) {
        let _ = self.wakeup_tx.try_send(WakeupSignal::Explicit {
            reason: reason.into(),
        });
    }

    /// Stop the RunLoop.
    ///
    /// Similar to CFRunLoopStop.
    pub fn stop(&self) {
        self.set_state(RunLoopState::Stopping);
        let _ = self.wakeup_tx.try_send(WakeupSignal::Stop);
    }

    // ========================================================================
    // Run Methods
    // ========================================================================

    /// Run the RunLoop (blocking until stopped).
    pub async fn run(&self) -> RunLoopResult<()> {
        self.run_in_mode(RunLoopMode::Default, Duration::MAX).await?;
        Ok(())
    }

    /// Run the RunLoop in a specific mode.
    ///
    /// Returns when stopped, timed out, or error.
    pub async fn run_in_mode(
        &self,
        mode: RunLoopMode,
        timeout: Duration,
    ) -> RunLoopResult<RunLoopRunResult> {
        let deadline = Instant::now() + timeout;

        // Set current mode
        *self.current_mode.write().await = mode.clone();
        self.set_state(RunLoopState::Running);
        self.metrics.mark_start();

        // Get mode data
        let mode_data = self
            .modes
            .get(&mode)
            .ok_or(RunLoopError::ModeNotFound(mode.clone()))?;

        // ┌─────────────────────────────────────────────────────────────┐
        // │ 1. Notify Entry                                             │
        // └─────────────────────────────────────────────────────────────┘
        debug!("RunLoop: Entry");
        self.notify_observers(RunLoopPhase::Entry, &mode).await;

        loop {
            self.metrics.record_iteration();

            // Check stop conditions
            if self.state() == RunLoopState::Stopping {
                break;
            }
            if Instant::now() >= deadline {
                self.notify_observers(RunLoopPhase::Exit, &mode).await;
                return Ok(RunLoopRunResult::TimedOut);
            }

            let process_start = Instant::now();

            // ┌─────────────────────────────────────────────────────────┐
            // │ 2. Notify BeforeTimers, promote delayed tasks           │
            // └─────────────────────────────────────────────────────────┘
            debug!("RunLoop: BeforeTimers");
            self.notify_observers(RunLoopPhase::BeforeTimers, &mode)
                .await;
            self.task_queue.promote_delayed().await;

            // ┌─────────────────────────────────────────────────────────┐
            // │ 3. Notify BeforeSources, process signaled Source0s      │
            // └─────────────────────────────────────────────────────────┘
            debug!("RunLoop: BeforeSources");
            self.notify_observers(RunLoopPhase::BeforeSources, &mode)
                .await;
            let source0_tasks = self.process_sources0(&mode_data).await?;
            for task in source0_tasks {
                self.task_queue.enqueue(task).await?;
            }

            // ┌─────────────────────────────────────────────────────────┐
            // │ 4. Check Source1 messages (non-blocking)                │
            // └─────────────────────────────────────────────────────────┘
            if let Some(tasks) = self.try_process_source1().await? {
                for task in tasks {
                    self.task_queue.enqueue(task).await?;
                }
                continue; // Don't sleep if there was activity
            }

            // ┌─────────────────────────────────────────────────────────┐
            // │ 5. Process pending tasks                                │
            // └─────────────────────────────────────────────────────────┘
            if let Some(task) = self.task_queue.dequeue().await {
                debug!("Processing task: {} (type: {})", task.id, task.task_type);
                self.metrics.record_events_processed(1);
                // Task processing would be handled by the Agent Driver
                // For now, we just log it
                continue;
            }

            self.metrics
                .record_process_time(process_start.elapsed().as_micros() as u64);

            // ┌─────────────────────────────────────────────────────────┐
            // │ 6. No tasks, notify BeforeWaiting, prepare to sleep     │
            // │    Key phase: batch commit, checkpoint, cleanup         │
            // └─────────────────────────────────────────────────────────┘
            debug!("RunLoop: BeforeWaiting");
            self.notify_observers(RunLoopPhase::BeforeWaiting, &mode)
                .await;
            self.set_state(RunLoopState::Waiting);

            // Clean up non-repeating observers
            self.cleanup_observers(&mode).await;

            // ┌─────────────────────────────────────────────────────────┐
            // │ 7. Sleep and wait for wakeup                            │
            // │    Wakeup: Source1 message, explicit wakeup, timeout    │
            // └─────────────────────────────────────────────────────────┘
            let wait_start = Instant::now();
            let wakeup = self.wait_for_wakeup(deadline).await;
            self.metrics
                .record_wait_time(wait_start.elapsed().as_micros() as u64);

            // ┌─────────────────────────────────────────────────────────┐
            // │ 8. Woken up, notify AfterWaiting                        │
            // └─────────────────────────────────────────────────────────┘
            self.set_state(RunLoopState::Running);
            debug!("RunLoop: AfterWaiting (wakeup: {:?})", wakeup);
            self.notify_observers(RunLoopPhase::AfterWaiting, &mode)
                .await;

            // ┌─────────────────────────────────────────────────────────┐
            // │ 9. Handle wakeup reason                                 │
            // └─────────────────────────────────────────────────────────┘
            match wakeup {
                WakeupSignal::Stop => break,
                WakeupSignal::SourceReady { source_id, message } => {
                    debug!("Source1 ready: {}", source_id);
                    let tasks = self.handle_source1_message(&source_id, message).await?;
                    for task in tasks {
                        self.task_queue.enqueue(task).await?;
                    }
                }
                WakeupSignal::Explicit { reason } => {
                    debug!("Explicit wakeup: {}", reason);
                }
            }
        }

        // ┌─────────────────────────────────────────────────────────────┐
        // │ 10. Notify Exit                                             │
        // └─────────────────────────────────────────────────────────────┘
        self.set_state(RunLoopState::Stopping);
        debug!("RunLoop: Exit");
        self.notify_observers(RunLoopPhase::Exit, &mode).await;
        self.set_state(RunLoopState::Stopped);

        info!("RunLoop stopped");
        Ok(RunLoopRunResult::Stopped)
    }

    /// Notify observers of a phase.
    async fn notify_observers(&self, phase: RunLoopPhase, mode: &RunLoopMode) {
        // Global observers
        {
            let observers = self.global_observers.read().await;
            for handle in observers.iter() {
                if handle.should_trigger(phase) {
                    self.metrics.record_observer_notification();
                    handle.observer().on_phase(phase, self).await;
                    handle.mark_fired();
                }
            }
        }

        // Mode-specific observers
        if let Some(mode_data) = self.modes.get(mode) {
            let observers = mode_data.observers.read().await;
            for handle in observers.iter() {
                if handle.should_trigger(phase) {
                    self.metrics.record_observer_notification();
                    handle.observer().on_phase(phase, self).await;
                    handle.mark_fired();
                }
            }
        }
    }

    /// Clean up non-repeating observers.
    async fn cleanup_observers(&self, mode: &RunLoopMode) {
        self.global_observers
            .write()
            .await
            .retain(|h| !h.should_remove());

        if let Some(mode_data) = self.modes.get(mode) {
            mode_data
                .observers
                .write()
                .await
                .retain(|h| !h.should_remove());
        }
    }

    /// Process signaled Source0s.
    async fn process_sources0(&self, mode_data: &ModeData) -> RunLoopResult<Vec<Task>> {
        let mut tasks = Vec::new();

        let sources = mode_data.sources0.read().await;
        for source in sources.iter() {
            if !source.is_valid() {
                continue;
            }
            if source.is_signaled() {
                self.metrics.record_source0_perform();
                match source.perform().await {
                    Ok(source_tasks) => {
                        tasks.extend(source_tasks);
                    }
                    Err(e) => {
                        warn!("Source0 {} perform error: {}", source.id(), e);
                    }
                }
            }
        }

        Ok(tasks)
    }

    /// Try to process Source1 messages (non-blocking).
    async fn try_process_source1(&self) -> RunLoopResult<Option<Vec<Task>>> {
        let mut receivers = self.source1_receivers.write().await;

        for receiver in receivers.iter_mut() {
            if !receiver.source.is_valid() {
                continue;
            }

            match receiver.receiver.try_recv() {
                Ok(msg) => {
                    self.metrics.record_source1_message();
                    let tasks = receiver.source.handle(msg).await?;
                    return Ok(Some(tasks));
                }
                Err(mpsc::error::TryRecvError::Empty) => continue,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Mark source as invalid
                    receiver.source.cancel();
                }
            }
        }

        // Clean up invalid sources
        receivers.retain(|r| r.source.is_valid());

        Ok(None)
    }

    /// Handle a Source1 message.
    async fn handle_source1_message(
        &self,
        source_id: &str,
        message: PortMessage,
    ) -> RunLoopResult<Vec<Task>> {
        let receivers = self.source1_receivers.read().await;

        for receiver in receivers.iter() {
            if receiver.source.id() == source_id && receiver.source.is_valid() {
                return receiver.source.handle(message).await;
            }
        }

        Ok(Vec::new())
    }

    /// Wait for wakeup.
    async fn wait_for_wakeup(&self, deadline: Instant) -> WakeupSignal {
        // Calculate wait timeout
        let next_delayed = self.task_queue.next_delayed_time().await;
        let wait_timeout = self.calculate_wait_timeout(deadline, next_delayed);

        let mut wakeup_rx = self.wakeup_rx.write().await;

        tokio::select! {
            // Explicit wakeup signal
            Some(signal) = wakeup_rx.recv() => signal,

            // Source1 activity
            result = self.wait_source1_activity() => {
                match result {
                    Some((source_id, msg)) => WakeupSignal::SourceReady {
                        source_id,
                        message: msg,
                    },
                    None => WakeupSignal::Explicit {
                        reason: "source1_closed".to_string(),
                    },
                }
            }

            // Timeout
            _ = tokio::time::sleep(wait_timeout) => {
                WakeupSignal::Explicit {
                    reason: "timeout".to_string(),
                }
            }
        }
    }

    /// Calculate wait timeout.
    fn calculate_wait_timeout(
        &self,
        deadline: Instant,
        next_delayed: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Duration {
        let now = Instant::now();
        let to_deadline = deadline.saturating_duration_since(now);

        match next_delayed {
            Some(delayed_time) => {
                let delayed_instant = {
                    let now_utc = chrono::Utc::now();
                    let diff = delayed_time - now_utc;
                    if diff.num_milliseconds() <= 0 {
                        Duration::ZERO
                    } else {
                        Duration::from_millis(diff.num_milliseconds() as u64)
                    }
                };
                std::cmp::min(to_deadline, delayed_instant)
            }
            None => std::cmp::min(to_deadline, Duration::from_secs(1)), // Default 1s poll
        }
    }

    /// Wait for Source1 activity.
    async fn wait_source1_activity(&self) -> Option<(String, PortMessage)> {
        // This is a simplified version - in production, we'd use a more
        // sophisticated approach like a FuturesUnordered
        let mut receivers = self.source1_receivers.write().await;

        for receiver in receivers.iter_mut() {
            if !receiver.source.is_valid() {
                continue;
            }

            if let Ok(msg) = receiver.receiver.try_recv() {
                return Some((receiver.source.id().to_string(), msg));
            }
        }

        // Wait a bit to avoid busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
        None
    }
}

impl Default for RunLoop {
    fn default() -> Self {
        Self::new(RunLoopConfig::default())
    }
}

// Implement TaskSubmitter trait for RunLoop to allow direct task submission
// from extensions and tools without needing a separate adapter.
#[async_trait::async_trait]
impl autohands_protocols::extension::TaskSubmitter for RunLoop {
    async fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        correlation_id: Option<String>,
    ) -> Result<(), autohands_protocols::error::ExtensionError> {
        use crate::task::{Task, TaskPriority, TaskSource};

        // Create Task from parameters
        let mut task = Task::new(task_type.to_string(), payload.clone())
            .with_source(TaskSource::Custom("task_submitter".to_string()));

        // Map priority if present in payload
        if let Some(priority) = payload.get("priority") {
            if let Some(p) = priority.as_str() {
                task = task.with_priority(match p {
                    "low" => TaskPriority::Low,
                    "high" => TaskPriority::High,
                    "critical" => TaskPriority::Critical,
                    _ => TaskPriority::Normal,
                });
            }
        }

        // Copy correlation ID
        if let Some(ref cid) = correlation_id {
            task = task.with_correlation_id(cid.clone());
        }

        // Inject into RunLoop
        self.inject_task(task).await.map_err(|e| {
            autohands_protocols::error::ExtensionError::Custom(format!(
                "Failed to submit task: {}",
                e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn test_runloop_new() {
        let run_loop = RunLoop::default();
        assert_eq!(run_loop.state(), RunLoopState::Created);
    }

    #[tokio::test]
    async fn test_runloop_modes() {
        let run_loop = RunLoop::default();

        assert!(run_loop.modes.contains_key(&RunLoopMode::Default));
        assert!(run_loop.modes.contains_key(&RunLoopMode::AgentProcessing));
        assert!(run_loop.modes.contains_key(&RunLoopMode::Background));
    }

    #[tokio::test]
    async fn test_runloop_inject_task() {
        let run_loop = RunLoop::default();
        let task = Task::new("test:task", serde_json::json!({"key": "value"}));

        run_loop.inject_task(task).await.unwrap();
        assert_eq!(run_loop.pending_task_count().await, 1);
    }

    #[tokio::test]
    async fn test_runloop_stop() {
        let run_loop = Arc::new(RunLoop::default());
        let run_loop_clone = run_loop.clone();

        // Stop before run completes
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            run_loop_clone.stop();
        });

        let result = run_loop
            .run_in_mode(RunLoopMode::Default, Duration::from_secs(10))
            .await;
        assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));
    }

    #[tokio::test]
    async fn test_runloop_timeout() {
        let run_loop = RunLoop::default();

        let result = run_loop
            .run_in_mode(RunLoopMode::Default, Duration::from_millis(50))
            .await;
        assert!(matches!(result, Ok(RunLoopRunResult::TimedOut)));
    }

    #[tokio::test]
    async fn test_runloop_observer() {
        use crate::observer::RunLoopObserver;
        use async_trait::async_trait;

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        struct TestObserver {
            counter: Arc<AtomicU32>,
        }

        #[async_trait]
        impl RunLoopObserver for TestObserver {
            fn activities(&self) -> u32 {
                RunLoopPhase::Entry as u32 | RunLoopPhase::Exit as u32
            }

            async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        let run_loop = Arc::new(RunLoop::default());
        run_loop
            .add_observer("test", Arc::new(TestObserver { counter: counter_clone }))
            .await;

        let run_loop_clone = run_loop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            run_loop_clone.stop();
        });

        run_loop
            .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
            .await
            .unwrap();

        // Should have been called for Entry and Exit
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_runloop_wakeup() {
        let run_loop = Arc::new(RunLoop::default());
        let wakeup_tx = run_loop.wakeup_sender();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            wakeup_tx
                .send(WakeupSignal::Explicit {
                    reason: "test".to_string(),
                })
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            wakeup_tx.send(WakeupSignal::Stop).await.unwrap();
        });

        let result = run_loop
            .run_in_mode(RunLoopMode::Default, Duration::from_secs(10))
            .await;
        assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));
    }
}
