//! RunLoop state accessors, metrics, and control methods.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::info;

use autohands_core::registry::ChannelRegistry;

use crate::agent_driver::AgentEventHandler;
use crate::error::RunLoopResult;
use crate::metrics::RunLoopMetrics;
use crate::mode::{RunLoopMode, RunLoopState};
use crate::run_loop::{RunLoop, WakeupSignal};
use crate::spawner::SpawnerInner;
use crate::task::Task;
use crate::task_queue::TaskQueue;

impl RunLoop {
    /// Set the agent event handler for task processing.
    pub async fn set_handler(&self, handler: Arc<dyn AgentEventHandler>) {
        *self.handler.write().await = Some(handler);
        info!("RunLoop: Agent event handler configured");
    }

    /// Set the channel registry for sending responses.
    pub async fn set_channel_registry(&self, registry: Arc<ChannelRegistry>) {
        *self.channel_registry.write().await = Some(registry);
        info!("RunLoop: Channel registry configured");
    }

    /// Get current state.
    pub fn state(&self) -> RunLoopState {
        RunLoopState::from(self.state.load(Ordering::SeqCst))
    }

    /// Set state.
    pub(crate) fn set_state(&self, state: RunLoopState) {
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

    /// Inject a task into the queue.
    pub async fn inject_task(&self, task: Task) -> RunLoopResult<()> {
        self.task_queue.enqueue(task).await?;
        self.metrics.record_event_enqueued();
        Ok(())
    }

    /// Wakeup the RunLoop. Similar to CFRunLoopWakeUp.
    pub fn wakeup(&self, reason: impl Into<String>) {
        let _ = self.wakeup_tx.try_send(WakeupSignal::Explicit {
            reason: reason.into(),
        });
    }

    /// Stop the RunLoop. Similar to CFRunLoopStop.
    pub fn stop(&self) {
        self.set_state(RunLoopState::Stopping);
        let _ = self.wakeup_tx.try_send(WakeupSignal::Stop);
    }
}
