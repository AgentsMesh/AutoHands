    use super::*;
    use crate::config::RunLoopConfig;

    #[test]
    fn test_agent_source_new() {
        let source = AgentSource0::new("test-agent");
        assert_eq!(source.id(), "test-agent");
        assert!(!source.is_signaled());
        assert!(source.is_valid());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_source_inject() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let task = Task::new("test:task", serde_json::json!({"key": "value"}));
        source.inject(task, &run_loop);

        assert!(source.is_signaled());
        assert_eq!(source.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_agent_source_perform() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        source.inject(
            Task::new("task1", serde_json::Value::Null),
            &run_loop,
        );
        source.inject(
            Task::new("task2", serde_json::Value::Null),
            &run_loop,
        );

        let tasks = source.perform().await.unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(!source.is_signaled());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_source_cancel() {
        let source = AgentSource0::new("test-agent");
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        source.inject(
            Task::new("task", serde_json::Value::Null),
            &run_loop,
        );
        source.cancel();

        assert!(!source.is_valid());
        assert_eq!(source.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_injector() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let injector = AgentTaskInjector::new(source.clone(), run_loop);

        injector.inject(Task::new("test", serde_json::Value::Null));

        assert!(source.is_signaled());
        assert_eq!(source.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_agent_injector_child_task() {
        let source = Arc::new(AgentSource0::new("test-agent"));
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let injector = AgentTaskInjector::new(source, run_loop);

        let parent = Task::new("parent", serde_json::Value::Null)
            .with_correlation_id("chain-1");

        let child = injector.create_child_task(&parent, "child", serde_json::json!({}));

        assert_eq!(child.parent_id, Some(parent.id));
        assert_eq!(child.correlation_id, Some("chain-1".to_string()));
    }

    #[test]
    fn test_agent_source_with_modes() {
        let source = AgentSource0::new("test")
            .with_modes(vec![RunLoopMode::Background]);

        assert_eq!(source.modes().len(), 1);
        assert_eq!(source.modes()[0], RunLoopMode::Background);
    }
