//! Workflow orchestration module.
//!
//! Provides multi-step task orchestration capabilities:
//! - Workflow definitions (Sequential, Parallel, Conditional)
//! - Workflow execution with timeout support
//! - Step result tracking
//! - Task-driven coordination with RunLoop
//! - HTTP API routes for workflow management

mod definition;
mod executor;
mod executor_types;
mod mock_executor;
pub mod routes;
pub mod store;
mod workflow_composite;
mod workflow_steps;

pub use definition::{ExecutionState, StepType, Workflow, WorkflowExecution, WorkflowStep};
pub use executor::WorkflowExecutor;
pub use executor_types::{
    AgentExecutor, ConditionEvaluator, ExecutionContext, SimpleConditionEvaluator, StepResult,
};
pub use mock_executor::MockAgentExecutor;
pub use store::{MemoryWorkflowStore, WorkflowStore};
