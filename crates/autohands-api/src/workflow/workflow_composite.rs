//! Composite workflow step implementations (parallel, sequential, conditional).

use tracing::{debug, info};

use crate::error::InterfaceError;

use super::definition::WorkflowStep;
use super::executor::WorkflowExecutor;
use super::executor_types::{ExecutionContext, StepResult};

impl WorkflowExecutor {
    /// Execute parallel steps.
    pub(crate) async fn execute_parallel_steps(
        &self,
        step_id: &str,
        steps: &[WorkflowStep],
        context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!("Executing {} parallel steps in {}", steps.len(), step_id);

        let mut outputs = Vec::new();
        let mut all_success = true;
        let mut errors = Vec::new();

        let initial_context = context.clone();

        for step in steps {
            let mut step_context = initial_context.clone();
            match self.execute_step(step, &mut step_context).await {
                Ok(step_result) => {
                    if !step_result.success {
                        all_success = false;
                        if let Some(err) = &step_result.error {
                            errors.push(err.clone());
                        }
                    }
                    outputs.push(serde_json::json!({
                        "step_id": step_result.step_id,
                        "success": step_result.success,
                        "output": step_result.output,
                    }));
                    context.step_results.extend(step_context.step_results);
                    context.variables.extend(step_context.variables);
                }
                Err(e) => {
                    all_success = false;
                    errors.push(e.to_string());
                }
            }
        }

        if all_success {
            Ok(StepResult::success(step_id, serde_json::json!(outputs)))
        } else {
            Ok(StepResult::failure(step_id, errors.join("; ")))
        }
    }

    /// Execute sequential steps.
    pub(crate) async fn execute_sequential_steps(
        &self,
        step_id: &str,
        steps: &[WorkflowStep],
        context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!(
            "Executing {} sequential steps in {}",
            steps.len(),
            step_id
        );

        let mut outputs = Vec::new();

        for step in steps {
            let result = self.execute_step(step, context).await?;

            outputs.push(serde_json::json!({
                "step_id": result.step_id,
                "success": result.success,
                "output": result.output,
            }));

            if !result.success {
                return Ok(StepResult::failure(
                    step_id,
                    format!(
                        "Sequential step {} failed: {:?}",
                        result.step_id, result.error
                    ),
                ));
            }
        }

        Ok(StepResult::success(step_id, serde_json::json!(outputs)))
    }

    /// Execute a conditional step.
    pub(crate) async fn execute_conditional_step(
        &self,
        step_id: &str,
        condition: &str,
        if_true: &WorkflowStep,
        if_false: Option<&WorkflowStep>,
        context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!(
            "Evaluating condition for step {}: {}",
            step_id, condition
        );

        let condition_result = self
            .condition_evaluator
            .evaluate(condition, context)
            .await?;

        debug!(
            "Condition '{}' evaluated to: {}",
            condition, condition_result
        );

        if condition_result {
            let result = self.execute_step(if_true, context).await?;
            Ok(StepResult {
                step_id: step_id.to_string(),
                success: result.success,
                output: serde_json::json!({
                    "condition": condition,
                    "branch": "if_true",
                    "result": result.output,
                }),
                error: result.error,
                duration_ms: result.duration_ms,
            })
        } else if let Some(else_step) = if_false {
            let result = self.execute_step(else_step, context).await?;
            Ok(StepResult {
                step_id: step_id.to_string(),
                success: result.success,
                output: serde_json::json!({
                    "condition": condition,
                    "branch": "if_false",
                    "result": result.output,
                }),
                error: result.error,
                duration_ms: result.duration_ms,
            })
        } else {
            Ok(StepResult::success(
                step_id,
                serde_json::json!({
                    "condition": condition,
                    "branch": "none",
                    "result": null,
                }),
            ))
        }
    }
}
