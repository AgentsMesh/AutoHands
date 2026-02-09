//! Runtime integration for RunLoop.
//!
//! Provides the RuntimeAgentEventHandler that connects RunLoop events
//! to the actual AgentRuntime for agent execution.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, error, info};

use autohands_protocols::types::Message;
use autohands_runtime::AgentRuntime;

use crate::agent_driver::{AgentEventHandler, AgentResult};
use crate::agent_source::AgentTaskInjector;
use crate::error::{RunLoopError, RunLoopResult};
use crate::task::{Task, TaskPriority, TaskSource};

/// RuntimeAgentEventHandler - Connects RunLoop events to AgentRuntime.
///
/// This handler processes agent:execute, agent:subtask, and agent:delayed
/// events by delegating to the actual AgentRuntime.
pub struct RuntimeAgentEventHandler {
    /// Reference to AgentRuntime.
    runtime: Arc<AgentRuntime>,

    /// Default agent ID to use when not specified.
    default_agent: String,
}

impl RuntimeAgentEventHandler {
    /// Create a new RuntimeAgentEventHandler.
    pub fn new(runtime: Arc<AgentRuntime>, default_agent: impl Into<String>) -> Self {
        Self {
            runtime,
            default_agent: default_agent.into(),
        }
    }

    /// Extract agent ID from task payload.
    fn get_agent_id(&self, task: &Task) -> String {
        task
            .payload
            .get("agent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.default_agent.clone())
    }

    /// Extract prompt/message from task payload.
    fn get_prompt(&self, task: &Task) -> Option<String> {
        task
            .payload
            .get("prompt")
            .or_else(|| task.payload.get("task"))
            .or_else(|| task.payload.get("content"))
            .or_else(|| task.payload.get("message"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extract session ID from task.
    fn get_session_id(&self, task: &Task) -> String {
        task
            .payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| task.correlation_id.clone())
            .unwrap_or_else(|| task.id.to_string())
    }

    /// Execute agent and convert result to AgentResult.
    async fn execute_agent(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        let agent_id = self.get_agent_id(task);
        let prompt = self.get_prompt(task).ok_or_else(|| {
            RunLoopError::TaskProcessingError("Missing prompt in task payload".to_string())
        })?;
        let session_id = self.get_session_id(task);

        info!(
            "Executing agent: agent_id={}, session_id={}, task_id={}",
            agent_id, session_id, task.id
        );

        // Create user message from prompt
        let message = Message::user(&prompt);

        // Execute through AgentRuntime
        match self.runtime.execute(&agent_id, &session_id, message).await {
            Ok(messages) => {
                // Extract the final assistant response
                let response = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == autohands_protocols::types::MessageRole::Assistant)
                    .map(|m| m.content.text().to_string());

                debug!(
                    "Agent execution completed: session_id={}, messages={}",
                    session_id,
                    messages.len()
                );

                // Check if there are follow-up tasks to inject
                let follow_up_tasks = self.extract_follow_up_tasks(task, &messages, injector);

                Ok(AgentResult {
                    response,
                    tasks: follow_up_tasks,
                    is_complete: true,
                    error: None,
                })
            }
            Err(e) => {
                error!(
                    "Agent execution failed: agent_id={}, session_id={}, error={}",
                    agent_id, session_id, e
                );

                // Create error task for notification
                let error_task = self.create_error_task(task, &e.to_string());
                injector.inject(error_task);

                Ok(AgentResult::failed(e.to_string()))
            }
        }
    }

    /// Extract follow-up tasks from agent messages.
    ///
    /// This analyzes the agent's response for self-driving patterns:
    /// - Subtask decomposition
    /// - Delayed actions
    /// - Continuation tasks
    fn extract_follow_up_tasks(
        &self,
        parent_task: &Task,
        messages: &[Message],
        _injector: &AgentTaskInjector,
    ) -> Vec<Task> {
        let mut tasks = Vec::new();

        // Look for structured follow-up instructions in metadata
        for message in messages {
            if let Some(follow_ups) = message.metadata.get("follow_up_tasks") {
                if let Ok(parsed) = serde_json::from_value::<Vec<FollowUpTask>>(follow_ups.clone())
                {
                    for follow_up in parsed {
                        let task = self.create_follow_up_task(parent_task, follow_up);
                        tasks.push(task);
                    }
                }
            }
        }

        tasks
    }

    /// Create a follow-up task from agent instruction.
    fn create_follow_up_task(
        &self,
        parent_task: &Task,
        follow_up: FollowUpTask,
    ) -> Task {
        let mut task = Task::new(follow_up.task_type, follow_up.payload)
            .with_source(TaskSource::Agent)
            .with_parent(parent_task.id);

        // Inherit correlation ID for task chain tracking
        if let Some(ref correlation_id) = parent_task.correlation_id {
            task = task.with_correlation_id(correlation_id.clone());
        }

        // Set scheduled time for delayed tasks
        if let Some(delay_ms) = follow_up.delay_ms {
            let scheduled_at = chrono::Utc::now()
                + chrono::Duration::milliseconds(delay_ms as i64);
            task = task.with_scheduled_at(scheduled_at);
        }

        // Set priority
        if let Some(priority) = follow_up.priority {
            task.priority = priority;
        }

        task
    }

    /// Create an error task for notification.
    fn create_error_task(&self, original_task: &Task, error: &str) -> Task {
        let mut task = Task::new(
            "agent:error",
            serde_json::json!({
                "original_task_id": original_task.id.to_string(),
                "original_task_type": original_task.task_type,
                "error": error,
            }),
        )
        .with_source(TaskSource::System)
        .with_priority(TaskPriority::High);

        if let Some(ref correlation_id) = original_task.correlation_id {
            task = task.with_correlation_id(correlation_id.clone());
        }

        task
    }
}

/// Follow-up task instruction from agent.
#[derive(Debug, Clone, serde::Deserialize)]
struct FollowUpTask {
    /// Task type (e.g., "agent:subtask", "agent:delayed").
    task_type: String,

    /// Task payload.
    payload: serde_json::Value,

    /// Delay in milliseconds (for delayed tasks).
    delay_ms: Option<u64>,

    /// Task priority.
    priority: Option<TaskPriority>,
}

#[async_trait]
impl AgentEventHandler for RuntimeAgentEventHandler {
    async fn handle_execute(
        &self,
        event: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        self.execute_agent(event, injector).await
    }

    async fn handle_subtask(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        // Subtasks are also executed through the agent runtime
        // The difference is they're part of a task chain
        debug!("Handling subtask: task_id={}", task.id);
        self.execute_agent(task, injector).await
    }

    async fn handle_delayed(
        &self,
        task: &Task,
        injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        // Delayed tasks are executed when their scheduled time arrives
        debug!("Handling delayed task: task_id={}", task.id);
        self.execute_agent(task, injector).await
    }
}

/// Builder for RuntimeAgentEventHandler.
pub struct RuntimeAgentEventHandlerBuilder {
    runtime: Option<Arc<AgentRuntime>>,
    default_agent: String,
}

impl RuntimeAgentEventHandlerBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            runtime: None,
            default_agent: "general".to_string(),
        }
    }

    /// Set the AgentRuntime.
    pub fn runtime(mut self, runtime: Arc<AgentRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the default agent ID.
    pub fn default_agent(mut self, agent: impl Into<String>) -> Self {
        self.default_agent = agent.into();
        self
    }

    /// Build the handler.
    pub fn build(self) -> Result<RuntimeAgentEventHandler, &'static str> {
        let runtime = self.runtime.ok_or("AgentRuntime is required")?;
        Ok(RuntimeAgentEventHandler::new(runtime, self.default_agent))
    }
}

impl Default for RuntimeAgentEventHandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_core::registry::{ProviderRegistry, ToolRegistry};
    use autohands_runtime::AgentRuntimeConfig;

    fn create_test_runtime() -> Arc<AgentRuntime> {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentRuntimeConfig::default();
        Arc::new(AgentRuntime::new(provider_registry, tool_registry, config))
    }

    #[test]
    fn test_handler_creation() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandler::new(runtime, "general");
        assert_eq!(handler.default_agent, "general");
    }

    #[test]
    fn test_handler_builder() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandlerBuilder::new()
            .runtime(runtime)
            .default_agent("custom-agent")
            .build()
            .unwrap();

        assert_eq!(handler.default_agent, "custom-agent");
    }

    #[test]
    fn test_handler_builder_missing_runtime() {
        let result = RuntimeAgentEventHandlerBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_agent_id() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandler::new(runtime, "default-agent");

        // With agent in payload
        let event = Task::new(
            "agent:execute",
            serde_json::json!({ "agent": "specific-agent", "prompt": "test" }),
        );
        assert_eq!(handler.get_agent_id(&event), "specific-agent");

        // Without agent in payload
        let event = Task::new(
            "agent:execute",
            serde_json::json!({ "prompt": "test" }),
        );
        assert_eq!(handler.get_agent_id(&event), "default-agent");
    }

    #[test]
    fn test_get_prompt() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandler::new(runtime, "default");

        // prompt field
        let event = Task::new(
            "agent:execute",
            serde_json::json!({ "prompt": "do something" }),
        );
        assert_eq!(handler.get_prompt(&event), Some("do something".to_string()));

        // task field
        let event = Task::new(
            "agent:subtask",
            serde_json::json!({ "task": "subtask content" }),
        );
        assert_eq!(handler.get_prompt(&event), Some("subtask content".to_string()));

        // content field
        let event = Task::new(
            "agent:execute",
            serde_json::json!({ "content": "message content" }),
        );
        assert_eq!(handler.get_prompt(&event), Some("message content".to_string()));

        // No prompt
        let event = Task::new("agent:execute", serde_json::json!({}));
        assert_eq!(handler.get_prompt(&event), None);
    }

    #[test]
    fn test_get_session_id() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandler::new(runtime, "default");

        // With session_id in payload
        let event = Task::new(
            "agent:execute",
            serde_json::json!({ "session_id": "custom-session", "prompt": "test" }),
        );
        assert_eq!(handler.get_session_id(&event), "custom-session");

        // With correlation_id
        let event = Task::new("agent:execute", serde_json::json!({ "prompt": "test" }))
            .with_correlation_id("correlation-123");
        assert_eq!(handler.get_session_id(&event), "correlation-123");

        // Falls back to task ID
        let task = Task::new("agent:execute", serde_json::json!({ "prompt": "test" }));
        assert_eq!(handler.get_session_id(&task), task.id.to_string());
    }

    #[test]
    fn test_create_error_task() {
        let runtime = create_test_runtime();
        let handler = RuntimeAgentEventHandler::new(runtime, "default");

        let original_task = Task::new(
            "agent:execute",
            serde_json::json!({ "prompt": "test" }),
        )
        .with_correlation_id("chain-1");

        let error_task = handler.create_error_task(&original_task, "Test error");

        assert_eq!(error_task.task_type, "agent:error");
        assert_eq!(error_task.priority, TaskPriority::High);
        assert_eq!(error_task.correlation_id, Some("chain-1".to_string()));
        assert_eq!(
            error_task.payload.get("error").unwrap().as_str().unwrap(),
            "Test error"
        );
    }
}
