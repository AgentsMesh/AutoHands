//! Agent task source for self-driving agent behavior.
//!
//! This module implements the core mechanism for Agent self-driving:
//! agents can produce tasks that flow back into the RunLoop queue.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use tracing::debug;

use crate::error::RunLoopResult;
use crate::task::Task;
use crate::mode::RunLoopMode;
use crate::source::Source0;
use crate::RunLoop;

/// Agent task Source0.
///
/// This is the key component for implementing Agent self-driving.
/// Agents can inject tasks back into the RunLoop through this source.
pub struct AgentSource0 {
    /// Source ID.
    id: String,

    /// Pending tasks to be injected.
    pending_tasks: RwLock<VecDeque<Task>>,

    /// Whether the source has been signaled.
    signaled: AtomicBool,

    /// Whether the source is cancelled.
    cancelled: AtomicBool,

    /// Associated modes.
    modes: Vec<RunLoopMode>,
}

impl AgentSource0 {
    /// Create a new AgentSource0.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            pending_tasks: RwLock::new(VecDeque::new()),
            signaled: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Default, RunLoopMode::AgentProcessing],
        }
    }

    /// Create with custom modes.
    pub fn with_modes(mut self, modes: Vec<RunLoopMode>) -> Self {
        self.modes = modes;
        self
    }

    /// Inject a task into the pending queue.
    ///
    /// This is the core method for Agent self-driving.
    /// After injection, call signal() and RunLoop::wakeup() to trigger processing.
    pub fn inject(&self, task: Task, run_loop: &RunLoop) {
        {
            let mut tasks = self.pending_tasks.write();
            tasks.push_back(task);
        }

        // Signal + Wakeup (similar to CFRunLoopSourceSignal + CFRunLoopWakeUp)
        self.signal();
        run_loop.wakeup("agent:task_injected");

        debug!("Agent task injected, source signaled");
    }

    /// Inject multiple tasks at once.
    pub fn inject_batch(&self, tasks: Vec<Task>, run_loop: &RunLoop) {
        {
            let mut pending = self.pending_tasks.write();
            pending.extend(tasks);
        }

        self.signal();
        run_loop.wakeup("agent:tasks_injected");

        debug!("Agent tasks batch injected");
    }

    /// Get the number of pending tasks.
    pub fn pending_count(&self) -> usize {
        self.pending_tasks.read().len()
    }

    /// Clear all pending tasks.
    pub fn clear(&self) {
        self.pending_tasks.write().clear();
        self.clear_signal();
    }
}

#[async_trait]
impl Source0 for AgentSource0 {
    fn id(&self) -> &str {
        &self.id
    }

    fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::SeqCst)
    }

    fn signal(&self) {
        self.signaled.store(true, Ordering::SeqCst);
    }

    fn clear_signal(&self) {
        self.signaled.store(false, Ordering::SeqCst);
    }

    async fn perform(&self) -> RunLoopResult<Vec<Task>> {
        let tasks: Vec<Task> = {
            let mut pending = self.pending_tasks.write();
            std::mem::take(&mut *pending).into_iter().collect()
        };

        self.clear_signal();

        debug!("AgentSource0 performed, produced {} tasks", tasks.len());
        Ok(tasks)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.pending_tasks.write().clear();
        self.clear_signal();
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }
}

/// Agent task injector.
///
/// A convenient wrapper for injecting tasks from agents.
/// Holds a reference to both the source and the RunLoop.
pub struct AgentTaskInjector {
    source: Arc<AgentSource0>,
    run_loop: Arc<RunLoop>,
}

impl AgentTaskInjector {
    /// Create a new injector.
    pub fn new(source: Arc<AgentSource0>, run_loop: Arc<RunLoop>) -> Self {
        Self { source, run_loop }
    }

    /// Inject a task.
    pub fn inject(&self, task: Task) {
        self.source.inject(task, &self.run_loop);
    }

    /// Inject multiple tasks.
    pub fn inject_batch(&self, tasks: Vec<Task>) {
        self.source.inject_batch(tasks, &self.run_loop);
    }

    /// Create a child task with correlation.
    pub fn create_child_task(
        &self,
        parent: &Task,
        task_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Task {
        let mut task = Task::new(task_type, payload)
            .with_source(crate::task::TaskSource::Agent)
            .with_parent(parent.id);

        // Inherit correlation ID
        if let Some(ref correlation_id) = parent.correlation_id {
            task = task.with_correlation_id(correlation_id.clone());
        }

        task
    }
}

impl Clone for AgentTaskInjector {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            run_loop: self.run_loop.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RunLoopConfig;

    #[test]
    fn test_agent_source_new() {
        let source = AgentSource0::new("test-agent");
        assert_eq!(source.id(), "test-agent");
        assert!(!source.is_signaled());
        assert!(source.is_valid());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_source_inject() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let task = Task::new("test:task", serde_json::json!({"key": "value"}));
        source.inject(task, &run_loop);

        assert!(source.is_signaled());
        assert_eq!(source.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_agent_source_perform() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        source.inject(
            Task::new("task1", serde_json::Value::Null),
            &run_loop,
        );
        source.inject(
            Task::new("task2", serde_json::Value::Null),
            &run_loop,
        );

        let tasks = source.perform().await.unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(!source.is_signaled());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_source_cancel() {
        let source = AgentSource0::new("test-agent");
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        source.inject(
            Task::new("task", serde_json::Value::Null),
            &run_loop,
        );
        source.cancel();

        assert!(!source.is_valid());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_injector() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let injector = AgentTaskInjector::new(source.clone(), run_loop);

        injector.inject(Task::new("test", serde_json::Value::Null));

        assert!(source.is_signaled());
        assert_eq!(source.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_agent_injector_child_task() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let injector = AgentTaskInjector::new(source, run_loop);

        let parent = Task::new("parent", serde_json::Value::Null)
            .with_correlation_id("chain-1");

        let child = injector.create_child_task(&parent, "child", serde_json::json!({}));

        assert_eq!(child.parent_id, Some(parent.id));
        assert_eq!(child.correlation_id, Some("chain-1".to_string()));
    }

    #[test]
    fn test_agent_source_with_modes() {
        let source = AgentSource0::new("test")
            .with_modes(vec![RunLoopMode::Background]);

        assert_eq!(source.modes().len(), 1);
        assert_eq!(source.modes()[0], RunLoopMode::Background);
    }
}
