//! AgentDriver method implementations.

use std::time::Duration;

use chrono::Utc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::agent_source::AgentTaskInjector;
use crate::error::{RunLoopError, RunLoopResult};
use crate::task::{Task, TaskPriority, TaskSource};

use super::{AgentDriver, AgentEventHandler, AgentExecutionContext, AgentResult, ExecutionStatus};

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

impl AgentDriver {
    /// Start the driver.
    pub fn start(&self) {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("AgentDriver started with {} workers", self.config.workers.max_workers);
    }

    /// Stop the driver.
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("AgentDriver stopped");
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get active execution context count.
    pub fn active_contexts(&self) -> usize {
        self.contexts.len()
    }

    /// Get total tasks processed.
    pub fn total_tasks_processed(&self) -> u64 {
        self.tasks_processed.load(std::sync::atomic::Ordering::Relaxed)
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

        self.tasks_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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

    /// Get an execution context by ID.
    pub fn get_context(&self, context_id: &str) -> Option<AgentExecutionContext> {
        self.contexts.get(context_id).map(|c| c.clone())
    }

    /// Create a new execution context.
    pub fn create_context(
        &self,
        agent: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> String {
        let context_id = Uuid::new_v4().to_string();
        let context = AgentExecutionContext {
            id: context_id.clone(),
            agent: agent.into(),
            correlation_id: correlation_id.into(),
            started_at: Utc::now(),
            status: ExecutionStatus::Active,
            tasks_processed: 0,
        };

        self.contexts.insert(context_id.clone(), context);
        context_id
    }

    /// Update execution context status.
    pub fn update_context_status(&self, context_id: &str, status: ExecutionStatus) {
        if let Some(mut context) = self.contexts.get_mut(context_id) {
            context.status = status;
        }
    }

    /// Remove an execution context.
    pub fn remove_context(&self, context_id: &str) {
        self.contexts.remove(context_id);
    }
}
