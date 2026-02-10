
    use super::*;

    #[test]
    fn test_runloop_runner_builder() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());

        let runner = RunLoopDaemonBuilder::new()
            .provider_registry(provider_registry)
            .tool_registry(tool_registry)
            .default_agent("test-agent")
            .build()
            .unwrap();

        assert_eq!(runner.default_agent, "test-agent");
    }

    #[test]
    fn test_runloop_runner_builder_missing_registry() {
        let result = RunLoopDaemonBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_runloop_runner_creation() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());

        let runner =
            RunLoopRunner::new(provider_registry, tool_registry).with_default_agent("custom");

        assert_eq!(runner.default_agent, "custom");
    }
