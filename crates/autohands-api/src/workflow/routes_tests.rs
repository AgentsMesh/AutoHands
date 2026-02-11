//! Tests for workflow routes.

use super::*;
use crate::workflow::definition::WorkflowStep;

#[test]
fn test_workflow_response_serialization() {
    let step = WorkflowStep::agent("s1", "Step 1", "test-agent", "Do something");
    let workflow = Workflow::new("wf-1", "Test Workflow", step);
    let response = WorkflowResponse { workflow };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["workflow"]["id"], "wf-1");
    assert_eq!(json["workflow"]["name"], "Test Workflow");
}

#[test]
fn test_workflow_list_response_serialization() {
    let response = WorkflowListResponse {
        count: 0,
        workflows: vec![],
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["count"], 0);
    assert!(json["workflows"].as_array().unwrap().is_empty());
}

#[test]
fn test_workflow_run_response_serialization() {
    let response = WorkflowRunResponse {
        execution_id: "exec-1".to_string(),
        workflow_id: "wf-1".to_string(),
        status: "Completed".to_string(),
        error: None,
        step_results: serde_json::json!({}),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["execution_id"], "exec-1");
    assert_eq!(json["status"], "Completed");
    assert!(json.get("error").is_none());
}
