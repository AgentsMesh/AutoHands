//! Tests for workflow executor.

use super::*;
use crate::workflow::executor_types::{ExecutionContext, StepResult};
use crate::workflow::mock_executor::MockAgentExecutor;

#[tokio::test]
async fn test_execute_agent_step() {
    let executor = Arc::new(MockAgentExecutor::new());
    executor
        .set_response("test-agent", serde_json::json!({"result": "success"}))
        .await;

    let workflow_executor = WorkflowExecutor::new(executor);
    let mut context = ExecutionContext::new();

    let step = WorkflowStep::agent("step1", "Test Step", "test-agent", "Do something");
    let result = workflow_executor
        .execute_step(&step, &mut context)
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(result.output["result"], "success");
}

#[tokio::test]
async fn test_simple_condition_evaluator() {
    let evaluator = SimpleConditionEvaluator;
    let mut context = ExecutionContext::new();

    context.set("status", serde_json::json!("active"));
    assert!(evaluator
        .evaluate("status == active", &context)
        .await
        .unwrap());
    assert!(!evaluator
        .evaluate("status == inactive", &context)
        .await
        .unwrap());

    assert!(evaluator
        .evaluate("status != inactive", &context)
        .await
        .unwrap());
    assert!(!evaluator
        .evaluate("status != active", &context)
        .await
        .unwrap());

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
