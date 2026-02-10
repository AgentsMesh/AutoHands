
    use super::*;

    #[test]
    fn test_message_response_from() {
        let msg = Message::user("Hello");
        let resp: MessageResponse = (&msg).into();
        assert_eq!(resp.role, "user");
        assert_eq!(resp.content, "Hello");
    }

    #[test]
    fn test_agent_run_request_deserialize() {
        let json = r#"{"task": "list files", "model": "test-model"}"#;
        let req: AgentRunRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "list files");
        assert_eq!(req.model, Some("test-model".to_string()));
        assert!(req.session_id.is_none());
    }

    #[test]
    fn test_agent_run_response_serialize() {
        let resp = AgentRunResponse {
            session_id: "test-session".to_string(),
            messages: vec![],
            status: "completed".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-session"));
        assert!(json.contains("completed"));
        assert!(!json.contains("error")); // Should be skipped when None
    }

    #[test]
    fn test_tools_list_response_serialize() {
        let resp = ToolsListResponse {
            count: 2,
            tools: vec![
                ToolInfo {
                    name: "read_file".to_string(),
                    description: "Read a file".to_string(),
                },
                ToolInfo {
                    name: "write_file".to_string(),
                    description: "Write a file".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("read_file"));
        assert!(json.contains("write_file"));
    }

    #[test]
    fn test_agent_info_from_config() {
        let config = AgentConfig::new("test-agent", "Test Agent", "test-model");
        let info: AgentInfo = (&config).into();
        assert_eq!(info.id, "test-agent");
        assert_eq!(info.name, "Test Agent");
        assert_eq!(info.default_model, "test-model");
    }
