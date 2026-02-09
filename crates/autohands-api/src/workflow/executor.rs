//! Workflow step executor.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::InterfaceError;

use super::definition::{ExecutionState, StepType, Workflow, WorkflowExecution, WorkflowStep};

/// Result of executing a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step ID that was executed.
    pub step_id: String,
    /// Whether the step succeeded.
    pub success: bool,
    /// Output data from the step.
    pub output: serde_json::Value,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

impl StepResult {
    /// Create a successful result.
    pub fn success(step_id: impl Into<String>, output: serde_json::Value) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output,
            error: None,
            duration_ms: 0,
        }
    }

    /// Create a failed result.
    pub fn failure(step_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            duration_ms: 0,
        }
    }

    /// Set duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Execution context passed between steps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Variables/outputs from previous steps.
    pub variables: HashMap<String, serde_json::Value>,
    /// Step results by step ID.
    pub step_results: HashMap<String, StepResult>,
    /// Current execution metadata.
    pub metadata: serde_json::Value,
}

impl ExecutionContext {
    /// Create a new context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(key.into(), value);
    }

    /// Get a variable.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// Record a step result.
    pub fn record_result(&mut self, result: StepResult) {
        let step_id = result.step_id.clone();
        if result.success {
            self.variables.insert(step_id.clone(), result.output.clone());
        }
        self.step_results.insert(step_id, result);
    }
}

/// Trait for executing agent steps.
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute an agent with the given prompt.
    async fn execute(
        &self,
        agent: &str,
        prompt: &str,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, InterfaceError>;
}

/// Trait for evaluating conditions.
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// Evaluate a condition expression.
    async fn evaluate(
        &self,
        condition: &str,
        context: &ExecutionContext,
    ) -> Result<bool, InterfaceError>;
}

/// Default condition evaluator using simple expression parsing.
pub struct SimpleConditionEvaluator;

#[async_trait]
impl ConditionEvaluator for SimpleConditionEvaluator {
    async fn evaluate(
        &self,
        condition: &str,
        context: &ExecutionContext,
    ) -> Result<bool, InterfaceError> {
        let condition = condition.trim();

        // Check for equality
        if let Some((left, right)) = condition.split_once("==") {
            let left = left.trim();
            let right = right.trim().trim_matches('"');
            if let Some(value) = context.get(left) {
                return Ok(value.as_str().is_some_and(|v| v == right)
                    || value.to_string().trim_matches('"') == right);
            }
            return Ok(false);
        }

        // Check for inequality
        if let Some((left, right)) = condition.split_once("!=") {
            let left = left.trim();
            let right = right.trim().trim_matches('"');
            if let Some(value) = context.get(left) {
                return Ok(value.as_str().map_or(true, |v| v != right)
                    && value.to_string().trim_matches('"') != right);
            }
            return Ok(true);
        }

        // Check for boolean/truthy variable
        if let Some(value) = context.get(condition) {
            return Ok(value.as_bool().unwrap_or_else(|| {
                !value.is_null() && value.as_str().map_or(true, |s| !s.is_empty())
            }));
        }

        // Check step success
        if let Some(result) = context.step_results.get(condition) {
            return Ok(result.success);
        }

        Ok(false)
    }
}

/// Workflow executor that runs workflow steps.
///
/// Note: WaitForEvent steps currently use a placeholder implementation.
/// In the future, this should integrate with RunLoop for event subscription.
pub struct WorkflowExecutor {
    /// Agent executor for running agents.
    agent_executor: Arc<dyn AgentExecutor>,
    /// Condition evaluator.
    condition_evaluator: Arc<dyn ConditionEvaluator>,
    /// Default timeout for steps.
    default_timeout: Duration,
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
    pub fn with_condition_evaluator(mut self, evaluator: Arc<dyn ConditionEvaluator>) -> Self {
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
        info!("Starting workflow execution: {} ({})", workflow.id, execution.id);

        let mut context = ExecutionContext::new();
        execution.state = ExecutionState::Running;

        let timeout = workflow
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = match tokio::time::timeout(timeout, self.execute_step(&workflow.root, &mut context)).await {
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
                    error!("Workflow {} failed: {:?}", workflow.id, step_result.error);
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
        execution.step_results = serde_json::to_value(&context.step_results).unwrap_or_default();

        Ok(context)
    }

    /// Execute a single step (boxed for recursion).
    pub fn execute_step<'a>(
        &'a self,
        step: &'a WorkflowStep,
        context: &'a mut ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<StepResult, InterfaceError>> + Send + 'a>> {
        Box::pin(async move {
            debug!("Executing step: {} ({})", step.name, step.id);

            let start = std::time::Instant::now();
            let timeout = step
                .timeout_secs
                .map(Duration::from_secs)
                .unwrap_or(self.default_timeout);

            let result = match tokio::time::timeout(timeout, self.execute_step_inner(step, context)).await {
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
                    let step_result = StepResult::failure(&step.id, e.to_string()).with_duration(duration_ms);
                    context.record_result(step_result.clone());
                    Ok(step_result)
                }
            }
        })
    }

    /// Inner step execution without timeout wrapper.
    fn execute_step_inner<'a>(
        &'a self,
        step: &'a WorkflowStep,
        context: &'a mut ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<StepResult, InterfaceError>> + Send + 'a>> {
        Box::pin(async move {
            match &step.step_type {
                StepType::Agent { agent, prompt } => {
                    self.execute_agent_step(&step.id, agent, prompt, context).await
                }
                StepType::Parallel { steps } => {
                    self.execute_parallel_steps(&step.id, steps, context).await
                }
                StepType::Sequential { steps } => {
                    self.execute_sequential_steps(&step.id, steps, context).await
                }
                StepType::Conditional {
                    condition,
                    if_true,
                    if_false,
                } => {
                    self.execute_conditional_step(&step.id, condition, if_true, if_false.as_deref(), context)
                        .await
                }
                StepType::WaitForEvent {
                    event_type,
                    timeout_secs,
                } => {
                    self.execute_wait_for_event_step(&step.id, event_type, *timeout_secs, context)
                        .await
                }
            }
        })
    }

    /// Execute an agent step.
    async fn execute_agent_step(
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

    /// Execute parallel steps.
    async fn execute_parallel_steps(
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
    async fn execute_sequential_steps(
        &self,
        step_id: &str,
        steps: &[WorkflowStep],
        context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!("Executing {} sequential steps in {}", steps.len(), step_id);

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
                    format!("Sequential step {} failed: {:?}", result.step_id, result.error),
                ));
            }
        }

        Ok(StepResult::success(step_id, serde_json::json!(outputs)))
    }

    /// Execute a conditional step.
    async fn execute_conditional_step(
        &self,
        step_id: &str,
        condition: &str,
        if_true: &WorkflowStep,
        if_false: Option<&WorkflowStep>,
        context: &mut ExecutionContext,
    ) -> Result<StepResult, InterfaceError> {
        info!("Evaluating condition for step {}: {}", step_id, condition);

        let condition_result = self.condition_evaluator.evaluate(condition, context).await?;

        debug!("Condition '{}' evaluated to: {}", condition, condition_result);

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

    /// Execute a wait-for-event step.
    ///
    /// Note: This is currently a placeholder implementation.
    /// TODO: Integrate with RunLoop for proper event subscription.
    async fn execute_wait_for_event_step(
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
        // In the future, this should subscribe to RunLoop events
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
}

/// Mock agent executor for testing.
pub struct MockAgentExecutor {
    responses: RwLock<HashMap<String, serde_json::Value>>,
}

impl MockAgentExecutor {
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
        }
    }

    pub async fn set_response(&self, agent: &str, response: serde_json::Value) {
        self.responses.write().await.insert(agent.to_string(), response);
    }
}

impl Default for MockAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentExecutor for MockAgentExecutor {
    async fn execute(
        &self,
        agent: &str,
        prompt: &str,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, InterfaceError> {
        let responses = self.responses.read().await;

        if let Some(response) = responses.get(agent) {
            Ok(response.clone())
        } else {
            Ok(serde_json::json!({
                "agent": agent,
                "prompt": prompt,
                "status": "completed",
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_agent_step() {
        let executor = Arc::new(MockAgentExecutor::new());
        executor
            .set_response("test-agent", serde_json::json!({"result": "success"}))
            .await;

        let workflow_executor = WorkflowExecutor::new(executor);
        let mut context = ExecutionContext::new();

        let step = WorkflowStep::agent("step1", "Test Step", "test-agent", "Do something");
        let result = workflow_executor.execute_step(&step, &mut context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["result"], "success");
    }

    #[tokio::test]
    async fn test_simple_condition_evaluator() {
        let evaluator = SimpleConditionEvaluator;
        let mut context = ExecutionContext::new();

        context.set("status", serde_json::json!("active"));
        assert!(evaluator.evaluate("status == active", &context).await.unwrap());
        assert!(!evaluator.evaluate("status == inactive", &context).await.unwrap());

        assert!(evaluator.evaluate("status != inactive", &context).await.unwrap());
        assert!(!evaluator.evaluate("status != active", &context).await.unwrap());

        context.set("enabled", serde_json::json!(true));
        assert!(evaluator.evaluate("enabled", &context).await.unwrap());

        context.set("disabled", serde_json::json!(false));
        assert!(!evaluator.evaluate("disabled", &context).await.unwrap());
    }

    #[test]
    fn test_step_result_success() {
        let result = StepResult::success("step1", serde_json::json!({"data": "test"}));
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_step_result_failure() {
        let result = StepResult::failure("step1", "Something went wrong");
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Something went wrong"));
    }

    #[test]
    fn test_execution_context() {
        let mut context = ExecutionContext::new();
        context.set("key", serde_json::json!("value"));

        assert_eq!(context.get("key"), Some(&serde_json::json!("value")));
        assert_eq!(context.get("nonexistent"), None);
    }
}
