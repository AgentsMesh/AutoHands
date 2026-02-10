//! Workflow executor core - orchestrates workflow execution.

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, error, info, warn};

use crate::error::InterfaceError;

use super::definition::{ExecutionState, Workflow, WorkflowExecution, WorkflowStep};
use super::executor_types::{
    AgentExecutor, ConditionEvaluator, ExecutionContext, SimpleConditionEvaluator, StepResult,
};

/// Workflow executor that runs workflow steps.
///
/// Note: WaitForEvent steps currently use a placeholder implementation.
/// In the future, this should integrate with RunLoop for event subscription.
pub struct WorkflowExecutor {
    /// Agent executor for running agents.
    pub(crate) agent_executor: Arc<dyn AgentExecutor>,
    /// Condition evaluator.
    pub(crate) condition_evaluator: Arc<dyn ConditionEvaluator>,
    /// Default timeout for steps.
    pub(crate) default_timeout: Duration,
}

impl WorkflowExecutor {
    /// Create a new workflow executor.
    pub fn new(agent_executor: Arc<dyn AgentExecutor>) -> Self {
        Self {
            agent_executor,
            condition_evaluator: Arc::new(SimpleConditionEvaluator),
            default_timeout: Duration::from_secs(300),
        }
    }

    /// Set a custom condition evaluator.
    pub fn with_condition_evaluator(
        mut self,
        evaluator: Arc<dyn ConditionEvaluator>,
    ) -> Self {
        self.condition_evaluator = evaluator;
        self
    }

    /// Set default timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Execute a complete workflow.
    pub async fn execute_workflow(
        &self,
        workflow: &Workflow,
        execution: &mut WorkflowExecution,
    ) -> Result<ExecutionContext, InterfaceError> {
        info!(
            "Starting workflow execution: {} ({})",
            workflow.id, execution.id
        );

        let mut context = ExecutionContext::new();
        execution.state = ExecutionState::Running;

        let timeout = workflow
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = match tokio::time::timeout(
            timeout,
            self.execute_step(&workflow.root, &mut context),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                error!("Workflow {} timed out", workflow.id);
                execution.state = ExecutionState::Failed;
                execution.error = Some("Workflow timeout".to_string());
                return Err(InterfaceError::Timeout);
            }
        };

        match result {
            Ok(step_result) => {
                if step_result.success {
                    execution.state = ExecutionState::Completed;
                    info!("Workflow {} completed successfully", workflow.id);
                } else {
                    execution.state = ExecutionState::Failed;
                    execution.error = step_result.error.clone();
                    error!(
                        "Workflow {} failed: {:?}",
                        workflow.id, step_result.error
                    );
                }
                context.record_result(step_result);
            }
            Err(e) => {
                execution.state = ExecutionState::Failed;
                execution.error = Some(e.to_string());
                error!("Workflow {} execution error: {}", workflow.id, e);
                return Err(e);
            }
        }

        execution.ended_at = Some(chrono::Utc::now());
        execution.step_results =
            serde_json::to_value(&context.step_results).unwrap_or_default();

        Ok(context)
    }

    /// Execute a single step (boxed for recursion).
    pub fn execute_step<'a>(
        &'a self,
        step: &'a WorkflowStep,
        context: &'a mut ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<StepResult, InterfaceError>> + Send + 'a>>
    {
        Box::pin(async move {
            debug!("Executing step: {} ({})", step.name, step.id);

            let start = std::time::Instant::now();
            let timeout = step
                .timeout_secs
                .map(Duration::from_secs)
                .unwrap_or(self.default_timeout);

            let result = match tokio::time::timeout(
                timeout,
                self.execute_step_inner(step, context),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    warn!("Step {} timed out", step.id);
                    Ok(StepResult::failure(&step.id, "Step timeout"))
                }
            };

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(mut step_result) => {
                    step_result.duration_ms = duration_ms;
                    context.record_result(step_result.clone());
                    Ok(step_result)
                }
                Err(e) => {
                    let step_result = StepResult::failure(&step.id, e.to_string())
                        .with_duration(duration_ms);
                    context.record_result(step_result.clone());
                    Ok(step_result)
                }
            }
        })
    }

}
