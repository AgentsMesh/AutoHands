//! Workflow step execution - agent, event, and dispatch.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tracing::{debug, error, info, warn};

use crate::error::InterfaceError;

use super::definition::{StepType, WorkflowStep};
use super::executor::WorkflowExecutor;
use super::executor_types::{ExecutionContext, StepResult};

impl WorkflowExecutor {
    /// Execute an agent step.
    pub(crate) async fn execute_agent_step(
        &self,
        step_id: &str,
        agent: &str,
        prompt: &str,
        context: &ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!("Executing agent step: {} with agent '{}'", step_id, agent);

        match self.agent_executor.execute(agent, prompt, context).await {
            Ok(output) => {
                debug!("Agent step {} completed successfully", step_id);
                Ok(StepResult::success(step_id, output))
            }
            Err(e) => {
                error!("Agent step {} failed: {}", step_id, e);
                Ok(StepResult::failure(step_id, e.to_string()))
            }
        }
    }

    /// Execute a wait-for-event step.
    ///
    /// Note: This is currently a placeholder implementation.
    /// TODO: Integrate with RunLoop for proper event subscription.
    pub(crate) async fn execute_wait_for_event_step(
        &self,
        step_id: &str,
        event_type: &str,
        timeout_secs: Option<u64>,
        _context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        warn!(
            "WaitForEvent step {} is using placeholder implementation (event: {})",
            step_id, event_type
        );

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(60));

        // Placeholder: just wait a short time and return success
        tokio::time::sleep(Duration::from_millis(100).min(timeout)).await;

        Ok(StepResult::success(
            step_id,
            serde_json::json!({
                "event_type": event_type,
                "status": "placeholder",
                "message": "WaitForEvent is not yet integrated with RunLoop. Step completed as placeholder.",
            }),
        ))
    }

    /// Inner step execution without timeout wrapper.
    pub(crate) fn execute_step_inner<'a>(
        &'a self,
        step: &'a WorkflowStep,
        context: &'a mut ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<StepResult, InterfaceError>> + Send + 'a>>
    {
        Box::pin(async move {
            match &step.step_type {
                StepType::Agent { agent, prompt } => {
                    self.execute_agent_step(&step.id, agent, prompt, context)
                        .await
                }
                StepType::Parallel { steps } => {
                    self.execute_parallel_steps(&step.id, steps, context)
                        .await
                }
                StepType::Sequential { steps } => {
                    self.execute_sequential_steps(&step.id, steps, context)
                        .await
                }
                StepType::Conditional {
                    condition,
                    if_true,
                    if_false,
                } => {
                    self.execute_conditional_step(
                        &step.id,
                        condition,
                        if_true,
                        if_false.as_deref(),
                        context,
                    )
                    .await
                }
                StepType::WaitForEvent {
                    event_type,
                    timeout_secs,
                } => {
                    self.execute_wait_for_event_step(
                        &step.id,
                        event_type,
                        *timeout_secs,
                        context,
                    )
                    .await
                }
            }
        })
    }
}
