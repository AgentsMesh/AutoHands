//! Workflow orchestration module.
//!
//! Provides multi-step task orchestration capabilities:
//! - Workflow definitions (Sequential, Parallel, Conditional)
//! - Workflow execution with timeout support
//! - Step result tracking
//! - Task-driven coordination with RunLoop

mod definition;
mod executor;

pub use definition::{ExecutionState, StepType, Workflow, WorkflowExecution, WorkflowStep};
pub use executor::{
    AgentExecutor, ConditionEvaluator, ExecutionContext, MockAgentExecutor,
    SimpleConditionEvaluator, StepResult, WorkflowExecutor,
};
