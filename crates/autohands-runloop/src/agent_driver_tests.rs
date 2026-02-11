use super::*;

#[test]
fn test_agent_result() {
    let empty = AgentResult::empty();
    assert!(empty.response.is_none());
    assert!(!empty.is_complete);

    let completed = AgentResult::completed("done");
    assert_eq!(completed.response, Some("done".to_string()));
    assert!(completed.is_complete);

    let failed = AgentResult::failed("error");
    assert!(failed.error.is_some());
    assert!(failed.is_complete);
}

#[test]
fn test_agent_result_with_tasks() {
    let task = Task::new("agent:execute", serde_json::json!({"prompt": "test"}));
    let result = AgentResult::completed("done").with_tasks(vec![task]);
    assert_eq!(result.tasks.len(), 1);
    assert!(result.is_complete);
}

#[test]
fn test_execution_status() {
    assert_eq!(ExecutionStatus::Active, ExecutionStatus::Active);
    assert_ne!(ExecutionStatus::Active, ExecutionStatus::Completed);
}

#[test]
fn test_agent_execution_context() {
    let context = AgentExecutionContext {
        id: "ctx-1".to_string(),
        agent: "general".to_string(),
        correlation_id: "chain-1".to_string(),
        started_at: chrono::Utc::now(),
        status: ExecutionStatus::Active,
        tasks_processed: 0,
    };

    assert_eq!(context.id, "ctx-1");
    assert_eq!(context.agent, "general");
    assert_eq!(context.status, ExecutionStatus::Active);
}
