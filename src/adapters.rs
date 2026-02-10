//! Adapter types and utility functions for AutoHands.

use std::path::PathBuf;
use std::sync::Arc;

use autohands_checkpoint::CheckpointManager;
use autohands_monitor::metrics::MetricsRegistry;
use autohands_runtime::{CheckpointData, CheckpointSupport};

/// Get the default PID file path.
pub(crate) fn default_pid_file() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("autohands.pid"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands.pid"))
}

/// Get the .autohands directory path.
pub(crate) fn autohands_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands"))
        .unwrap_or_else(|| PathBuf::from(".autohands"))
}

/// Adapter: bridges CheckpointManager to CheckpointSupport trait.
///
/// CheckpointManager stores messages as serde_json::Value, while CheckpointSupport
/// uses Vec<Message>. This adapter handles the serialization/deserialization.
pub(crate) struct CheckpointAdapter {
    pub manager: Arc<CheckpointManager>,
}

#[async_trait::async_trait]
impl CheckpointSupport for CheckpointAdapter {
    fn should_checkpoint(&self, turn: u32) -> bool {
        self.manager.should_checkpoint(turn)
    }

    async fn create_checkpoint(
        &self,
        session_id: &str,
        turn: u32,
        messages: &[autohands_protocols::types::Message],
        context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages_json = serde_json::to_value(messages)?;
        self.manager.create(session_id, turn, messages_json, context.clone()).await?;
        Ok(())
    }

    async fn get_latest_checkpoint(
        &self,
        session_id: &str,
    ) -> Result<Option<CheckpointData>, Box<dyn std::error::Error + Send + Sync>> {
        match self.manager.get_latest(session_id).await? {
            Some(cp) => {
                let messages: Vec<autohands_protocols::types::Message> =
                    serde_json::from_value(cp.messages)?;
                Ok(Some(CheckpointData {
                    turn: cp.turn,
                    messages,
                    context: cp.context,
                }))
            }
            None => Ok(None),
        }
    }
}

/// Wraps an AgentEventHandler to add metrics instrumentation.
pub(crate) struct MetricsWrappedHandler {
    pub inner: Arc<autohands_runloop::RuntimeAgentEventHandler>,
    pub metrics: Arc<MetricsRegistry>,
    pub active_count: std::sync::atomic::AtomicU64,
}

impl MetricsWrappedHandler {
    /// Record task outcome: distinguishes Ok(AgentResult { error: Some }) as failed.
    async fn record_outcome(&self, result: &autohands_runloop::RunLoopResult<autohands_runloop::AgentResult>) {
        match result {
            Ok(agent_result) if agent_result.error.is_some() => {
                self.metrics.inc_counter("autohands_tasks_failed").await;
            }
            Ok(_) => {
                self.metrics.inc_counter("autohands_tasks_completed").await;
            }
            Err(_) => {
                self.metrics.inc_counter("autohands_tasks_failed").await;
            }
        }
    }
}

#[async_trait::async_trait]
impl autohands_runloop::AgentEventHandler for MetricsWrappedHandler {
    async fn handle_execute(
        &self,
        task: &autohands_runloop::Task,
        injector: &autohands_runloop::AgentTaskInjector,
    ) -> autohands_runloop::RunLoopResult<autohands_runloop::AgentResult> {
        use std::sync::atomic::Ordering;

        self.metrics.inc_counter("autohands_requests_total").await;
        let active = self.active_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.metrics.set_gauge("autohands_active_sessions", active).await;

        let result = self.inner.handle_execute(task, injector).await;

        let active = self.active_count.fetch_sub(1, Ordering::SeqCst) - 1;
        self.metrics.set_gauge("autohands_active_sessions", active).await;
        self.record_outcome(&result).await;
        result
    }

    async fn handle_subtask(
        &self,
        task: &autohands_runloop::Task,
        injector: &autohands_runloop::AgentTaskInjector,
    ) -> autohands_runloop::RunLoopResult<autohands_runloop::AgentResult> {
        self.metrics.inc_counter("autohands_requests_total").await;
        let result = self.inner.handle_subtask(task, injector).await;
        self.record_outcome(&result).await;
        result
    }

    async fn handle_delayed(
        &self,
        task: &autohands_runloop::Task,
        injector: &autohands_runloop::AgentTaskInjector,
    ) -> autohands_runloop::RunLoopResult<autohands_runloop::AgentResult> {
        self.metrics.inc_counter("autohands_requests_total").await;
        let result = self.inner.handle_delayed(task, injector).await;
        self.record_outcome(&result).await;
        result
    }
}
