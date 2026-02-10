//! Workflow executor types and traits.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::InterfaceError;

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
            self.variables
                .insert(step_id.clone(), result.output.clone());
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
