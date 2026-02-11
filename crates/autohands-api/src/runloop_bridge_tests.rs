
    use super::*;
    use autohands_protocols::channel::ReplyAddress;
    use autohands_runloop::RunLoopConfig;

    #[tokio::test]
    async fn test_runloop_state_creation() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let state = RunLoopState::from_runloop(run_loop);
        // Just verify it compiles and creates without panicking
        let _ = state;
    }

    #[test]
    fn test_runloop_task_request_deserialize() {
        let json = r#"{"task": "analyze code", "agent_id": "coder"}"#;
        let req: RunLoopTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "analyze code");
        assert_eq!(req.agent_id, Some("coder".to_string()));
        assert!(req.session_id.is_none());
    }

    #[test]
    fn test_runloop_task_response_serialize() {
        let resp = RunLoopTaskResponse {
            session_id: "test-session".to_string(),
            status: "queued".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-session"));
        assert!(json.contains("queued"));
        assert!(!json.contains("error")); // Should be skipped when None
    }

    #[tokio::test]
    async fn test_hybrid_state_creation() {
        let base = Arc::new(AppState::default());
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let runloop = Arc::new(RunLoopState::from_runloop(run_loop));
        let api_ws_channel = Arc::new(crate::websocket::ApiWsChannel::new());

        let hybrid = HybridAppState::new(base, runloop, api_ws_channel);
        assert!(Arc::strong_count(&hybrid.runloop) >= 1);
    }

    #[tokio::test]
    async fn test_runloop_state_submit_task() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let state = RunLoopState::from_runloop(run_loop.clone());

        // Inject a task via RunLoopState (no reply_to)
        let result = state
            .submit_task("test:event", serde_json::json!({"data": "test"}), None)
            .await;
        assert!(result.is_ok());

        // Verify task was added to the RunLoop's queue
        assert_eq!(run_loop.pending_task_count().await, 1);
    }

    #[tokio::test]
    async fn test_runloop_state_submit_task_with_reply_to() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let state = RunLoopState::from_runloop(run_loop.clone());

        let reply_to = ReplyAddress::with_thread("api-ws", "conn-123", "session-abc");
        let result = state
            .submit_task(
                "agent:execute",
                serde_json::json!({"prompt": "hello", "session_id": "session-abc"}),
                Some(reply_to),
            )
            .await;
        assert!(result.is_ok());

        // Verify task was added to the RunLoop's queue
        assert_eq!(run_loop.pending_task_count().await, 1);
    }

    #[tokio::test]
    async fn test_hybrid_state_api_ws_channel() {
        let base = Arc::new(AppState::default());
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let runloop = Arc::new(RunLoopState::from_runloop(run_loop));
        let api_ws_channel = Arc::new(crate::websocket::ApiWsChannel::new());

        let hybrid = HybridAppState::new(base, runloop, api_ws_channel.clone());
        assert_eq!(hybrid.api_ws_channel.connection_count(), 0);
        assert!(Arc::ptr_eq(&hybrid.api_ws_channel, &api_ws_channel));
    }
