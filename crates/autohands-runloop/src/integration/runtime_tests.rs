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
