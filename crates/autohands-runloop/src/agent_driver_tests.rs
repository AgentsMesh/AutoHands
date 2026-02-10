use super::*;
use std::time::Duration;

use crate::agent_source::AgentSource0;
use crate::config::RunLoopConfig;
use crate::RunLoop;

#[tokio::test]
async fn test_agent_driver_new() {
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let source = Arc::new(AgentSource0::new("agent"));
    let config = RunLoopConfig::default();

    let driver = AgentDriver::new(run_loop, source, config);
    assert!(!driver.is_running());
    assert_eq!(driver.active_contexts(), 0);
}

#[tokio::test]
async fn test_agent_driver_start_stop() {
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let source = Arc::new(AgentSource0::new("agent"));
    let config = RunLoopConfig::default();

    let driver = AgentDriver::new(run_loop, source, config);

    driver.start();
    assert!(driver.is_running());

    driver.stop();
    assert!(!driver.is_running());
}

#[tokio::test]
async fn test_agent_driver_create_tasks() {
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let source = Arc::new(AgentSource0::new("agent"));
    let config = RunLoopConfig::default();

    let driver = AgentDriver::new(run_loop, source, config);

    let execute = driver.create_execute_task("general", "test prompt");
    assert_eq!(execute.task_type, "agent:execute");

    let subtask = driver.create_subtask(&execute, "subtask");
    assert_eq!(subtask.task_type, "agent:subtask");
    assert_eq!(subtask.parent_id, Some(execute.id));

    let delayed = driver.create_delayed_task(&execute, "delayed", Duration::from_secs(5));
    assert_eq!(delayed.task_type, "agent:delayed");
    assert!(delayed.scheduled_at.is_some());
}

#[tokio::test]
async fn test_agent_driver_process_task() {
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let source = Arc::new(AgentSource0::new("agent"));
    let config = RunLoopConfig::default();

    let driver = AgentDriver::new(run_loop, source, config);
    driver.start();

    let task = driver.create_execute_task("general", "test");
    let result = driver.process_task(task).await.unwrap();

    assert!(result.is_complete);
    assert_eq!(driver.total_tasks_processed(), 1);
}

#[tokio::test]
async fn test_agent_driver_execution_context() {
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let source = Arc::new(AgentSource0::new("agent"));
    let config = RunLoopConfig::default();

    let driver = AgentDriver::new(run_loop, source, config);

    let context_id = driver.create_context("general", "chain-1");
    assert_eq!(driver.active_contexts(), 1);

    let context = driver.get_context(&context_id).unwrap();
    assert_eq!(context.agent, "general");
    assert_eq!(context.status, ExecutionStatus::Active);

    driver.update_context_status(&context_id, ExecutionStatus::Completed);
    let context = driver.get_context(&context_id).unwrap();
    assert_eq!(context.status, ExecutionStatus::Completed);

    driver.remove_context(&context_id);
    assert_eq!(driver.active_contexts(), 0);
}

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
