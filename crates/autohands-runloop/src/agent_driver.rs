//! Agent driver for task-driven agent execution.
//!
//! The AgentDriver integrates agent execution with the RunLoop,
//! implementing the Worker Pool pattern for concurrent execution.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::agent_source::{AgentTaskInjector, AgentSource0};
use crate::config::RunLoopConfig;
use crate::error::{RunLoopError, RunLoopResult};
use crate::task::{Task, TaskPriority, TaskSource};
use crate::RunLoop;

/// Agent execution session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Session ID.
    pub id: String,

    /// Agent name/type.
    pub agent: String,

    /// Correlation ID for task chain.
    pub correlation_id: String,

    /// Session start time.
    pub started_at: DateTime<Utc>,

    /// Current status.
    pub status: SessionStatus,

    /// Number of tasks processed.
    pub tasks_processed: u64,
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is active.
    Active,
    /// Session is paused.
    Paused,
    /// Session completed successfully.
    Completed,
    /// Session failed.
    Failed,
    /// Session was cancelled.
    Cancelled,
}

/// Agent execution result.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Response message (if any).
    pub response: Option<String>,

    /// Follow-up tasks produced (flow back to RunLoop).
    pub tasks: Vec<Task>,

    /// Whether the task is complete.
    pub is_complete: bool,

    /// Error message (if any).
    pub error: Option<String>,
}

impl AgentResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            response: None,
            tasks: Vec::new(),
            is_complete: false,
            error: None,
        }
    }

    /// Create a completed result.
    pub fn completed(response: impl Into<String>) -> Self {
        Self {
            response: Some(response.into()),
            tasks: Vec::new(),
            is_complete: true,
            error: None,
        }
    }

    /// Create a result with follow-up tasks.
    pub fn with_tasks(mut self, tasks: Vec<Task>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Create a failed result.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            response: None,
            tasks: Vec::new(),
            is_complete: true,
            error: Some(error.into()),
        }
    }
}

/// Agent task handler trait.
///
/// Implement this trait to define how agent tasks are processed.
#[async_trait::async_trait]
pub trait AgentEventHandler: Send + Sync {
    /// Handle an agent execution task.
    async fn handle_execute(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult>;

    /// Handle a subtask.
    async fn handle_subtask(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult>;

    /// Handle a delayed task.
    async fn handle_delayed(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult>;
}

/// Default no-op task handler.
pub struct NoOpEventHandler;

#[async_trait::async_trait]
impl AgentEventHandler for NoOpEventHandler {
    async fn handle_execute(
        &self,
        task: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        debug!("NoOp handler: execute task {}", task.id);
        Ok(AgentResult::completed("NoOp"))
    }

    async fn handle_subtask(
        &self,
        task: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        debug!("NoOp handler: subtask {}", task.id);
        Ok(AgentResult::completed("NoOp"))
    }

    async fn handle_delayed(
        &self,
        task: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        debug!("NoOp handler: delayed task {}", task.id);
        Ok(AgentResult::completed("NoOp"))
    }
}

/// Agent driver - integrates Agent execution with RunLoop.
///
/// Implements a Worker Pool pattern for concurrent agent execution.
pub struct AgentDriver {
    /// RunLoop reference.
    run_loop: Arc<RunLoop>,

    /// Agent task source.
    agent_source: Arc<AgentSource0>,

    /// Task handler.
    handler: Arc<dyn AgentEventHandler>,

    /// Worker pool semaphore.
    worker_semaphore: Arc<Semaphore>,

    /// Active sessions.
    sessions: DashMap<String, AgentSession>,

    /// Running flag.
    running: AtomicBool,

    /// Total tasks processed.
    tasks_processed: AtomicU64,

    /// Configuration.
    config: RunLoopConfig,
}

impl AgentDriver {
    /// Create a new AgentDriver.
    pub fn new(
        run_loop: Arc<RunLoop>,
        agent_source: Arc<AgentSource0>,
        config: RunLoopConfig,
    ) -> Self {
        Self {
            run_loop,
            agent_source,
            handler: Arc::new(NoOpEventHandler),
            worker_semaphore: Arc::new(Semaphore::new(config.workers.max_workers)),
            sessions: DashMap::new(),
            running: AtomicBool::new(false),
            tasks_processed: AtomicU64::new(0),
            config,
        }
    }

    /// Set the task handler.
    pub fn with_handler(mut self, handler: Arc<dyn AgentEventHandler>) -> Self {
        self.handler = handler;
        self
    }

    /// Start the driver.
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        info!("AgentDriver started with {} workers", self.config.workers.max_workers);
    }

    /// Stop the driver.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        info!("AgentDriver stopped");
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get active session count.
    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }

    /// Get total tasks processed.
    pub fn total_tasks_processed(&self) -> u64 {
        self.tasks_processed.load(Ordering::Relaxed)
    }

    /// Process a task.
    ///
    /// This is the main entry point for task processing.
    pub async fn process_task(&self, task: Task) -> RunLoopResult<AgentResult> {
        if !self.is_running() {
            return Err(RunLoopError::NotRunning);
        }

        // Acquire worker permit
        let _permit = self
            .worker_semaphore
            .acquire()
            .await
            .map_err(|e| RunLoopError::Internal(format!("Semaphore error: {}", e)))?;

        let injector = AgentTaskInjector::new(self.agent_source.clone(), self.run_loop.clone());

        let result = match task.task_type.as_str() {
            "agent:execute" => {
                info!(
                    "Agent execution started: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                self.handler.handle_execute(&task, &injector).await
            }
            "agent:subtask" => {
                debug!(
                    "Agent subtask started: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                self.handler.handle_subtask(&task, &injector).await
            }
            "agent:delayed" => {
                debug!(
                    "Agent delayed task: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                self.handler.handle_delayed(&task, &injector).await
            }
            _ => {
                debug!("Unknown task type: {}", task.task_type);
                Ok(AgentResult::empty())
            }
        };

        self.tasks_processed.fetch_add(1, Ordering::Relaxed);

        match &result {
            Ok(res) => {
                // Inject follow-up tasks
                if !res.tasks.is_empty() {
                    debug!("Injecting {} follow-up tasks", res.tasks.len());
                    injector.inject_batch(res.tasks.clone());
                }

                if res.is_complete {
                    info!(
                        "Agent execution completed: task_id={}, correlation_id={:?}",
                        task.id, task.correlation_id
                    );
                }
            }
            Err(e) => {
                error!(
                    "Agent execution failed: task_id={}, error={}",
                    task.id, e
                );
            }
        }

        result
    }

    /// Create an agent:execute task.
    pub fn create_execute_task(
        &self,
        agent: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Task {
        Task::new(
            "agent:execute",
            serde_json::json!({
                "agent": agent.into(),
                "prompt": prompt.into(),
            }),
        )
        .with_source(TaskSource::User)
        .with_priority(TaskPriority::Normal)
    }

    /// Create an agent:subtask task.
    pub fn create_subtask(
        &self,
        parent: &Task,
        subtask: impl Into<String>,
    ) -> Task {
        let mut task = Task::new(
            "agent:subtask",
            serde_json::json!({
                "task": subtask.into(),
            }),
        )
        .with_source(TaskSource::Agent)
        .with_parent(parent.id);

        if let Some(ref correlation_id) = parent.correlation_id {
            task = task.with_correlation_id(correlation_id.clone());
        }

        task
    }

    /// Create an agent:delayed task.
    pub fn create_delayed_task(
        &self,
        parent: &Task,
        subtask: impl Into<String>,
        delay: Duration,
    ) -> Task {
        let scheduled_at = Utc::now() + chrono::Duration::from_std(delay).unwrap();

        let mut task = Task::new(
            "agent:delayed",
            serde_json::json!({
                "task": subtask.into(),
            }),
        )
        .with_source(TaskSource::Agent)
        .with_scheduled_at(scheduled_at)
        .with_parent(parent.id);

        if let Some(ref correlation_id) = parent.correlation_id {
            task = task.with_correlation_id(correlation_id.clone());
        }

        task
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<AgentSession> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    /// Create a new session.
    pub fn create_session(&self, agent: impl Into<String>, correlation_id: impl Into<String>) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = AgentSession {
            id: session_id.clone(),
            agent: agent.into(),
            correlation_id: correlation_id.into(),
            started_at: Utc::now(),
            status: SessionStatus::Active,
            tasks_processed: 0,
        };

        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    /// Update session status.
    pub fn update_session_status(&self, session_id: &str, status: SessionStatus) {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.status = status;
        }
    }

    /// Remove a session.
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_driver_new() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let source = Arc::new(AgentSource0::new("agent"));
        let config = RunLoopConfig::default();

        let driver = AgentDriver::new(run_loop, source, config);
        assert!(!driver.is_running());
        assert_eq!(driver.active_sessions(), 0);
    }

    #[tokio::test]
    async fn test_agent_driver_start_stop() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let source = Arc::new(AgentSource0::new("agent"));
        let config = RunLoopConfig::default();

        let driver = AgentDriver::new(run_loop, source, config);

        driver.start();
        assert!(driver.is_running());

        driver.stop();
        assert!(!driver.is_running());
    }

    #[tokio::test]
    async fn test_agent_driver_create_tasks() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let source = Arc::new(AgentSource0::new("agent"));
        let config = RunLoopConfig::default();

        let driver = AgentDriver::new(run_loop, source, config);

        let execute = driver.create_execute_task("general", "test prompt");
        assert_eq!(execute.task_type, "agent:execute");

        let subtask = driver.create_subtask(&execute, "subtask");
        assert_eq!(subtask.task_type, "agent:subtask");
        assert_eq!(subtask.parent_id, Some(execute.id));

        let delayed = driver.create_delayed_task(&execute, "delayed", Duration::from_secs(5));
        assert_eq!(delayed.task_type, "agent:delayed");
        assert!(delayed.scheduled_at.is_some());
    }

    #[tokio::test]
    async fn test_agent_driver_process_task() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let source = Arc::new(AgentSource0::new("agent"));
        let config = RunLoopConfig::default();

        let driver = AgentDriver::new(run_loop, source, config);
        driver.start();

        let task = driver.create_execute_task("general", "test");
        let result = driver.process_task(task).await.unwrap();

        assert!(result.is_complete);
        assert_eq!(driver.total_tasks_processed(), 1);
    }

    #[tokio::test]
    async fn test_agent_driver_session() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let source = Arc::new(AgentSource0::new("agent"));
        let config = RunLoopConfig::default();

        let driver = AgentDriver::new(run_loop, source, config);

        let session_id = driver.create_session("general", "chain-1");
        assert_eq!(driver.active_sessions(), 1);

        let session = driver.get_session(&session_id).unwrap();
        assert_eq!(session.agent, "general");
        assert_eq!(session.status, SessionStatus::Active);

        driver.update_session_status(&session_id, SessionStatus::Completed);
        let session = driver.get_session(&session_id).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);

        driver.remove_session(&session_id);
        assert_eq!(driver.active_sessions(), 0);
    }

    #[test]
    fn test_agent_result() {
        let empty = AgentResult::empty();
        assert!(empty.response.is_none());
        assert!(!empty.is_complete);

        let completed = AgentResult::completed("done");
        assert_eq!(completed.response, Some("done".to_string()));
        assert!(completed.is_complete);

        let failed = AgentResult::failed("error");
        assert!(failed.error.is_some());
        assert!(failed.is_complete);
    }
}
