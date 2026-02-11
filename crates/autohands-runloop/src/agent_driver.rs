//! Agent types for task-driven agent execution.
//!
//! Defines the core types and traits for agent task processing:
//! - [`AgentEventHandler`]: Trait for handling agent tasks
//! - [`AgentResult`]: Result of agent task execution
//! - [`AgentExecutionContext`] / [`ExecutionStatus`]: Runtime state tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent_source::AgentTaskInjector;
use crate::error::RunLoopResult;
use crate::task::Task;

#[cfg(test)]
#[path = "agent_driver_tests.rs"]
mod tests;

/// Agent execution context - tracks the state of an agent execution session.
///
/// # Note on Session Management
///
/// This struct tracks runtime state for agent execution. For persistent
/// session data storage, use `Session` from `autohands-runtime`.
///
/// The distinction:
/// - `AgentExecutionContext`: Runtime state (status, tasks_processed, started_at)
/// - `Session`: Persistent data storage (key-value pairs, last_active)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionContext {
    /// Execution context ID.
    pub id: String,

    /// Agent name/type.
    pub agent: String,

    /// Correlation ID for task chain.
    pub correlation_id: String,

    /// Execution start time.
    pub started_at: DateTime<Utc>,

    /// Current status.
    pub status: ExecutionStatus,

    /// Number of tasks processed.
    pub tasks_processed: u64,
}

/// Execution status for an agent context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution is active.
    Active,
    /// Execution is paused.
    Paused,
    /// Execution completed successfully.
    Completed,
    /// Execution failed.
    Failed,
    /// Execution was cancelled.
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
/// The RunLoop dispatches tasks to the configured handler based on task type.
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
