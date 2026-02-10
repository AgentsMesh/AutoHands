    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_task_from_message() {
        let reply_to = ReplyAddress::new("web", "conn-123");
        let msg = InboundMessage {
            id: "msg-1".to_string(),
            content: "Hello, agent!".to_string(),
            reply_to: reply_to.clone(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
            attachments: Vec::new(),
        };

        let task = create_task_from_message(msg);

        assert_eq!(task.task_type, "agent:execute");
        assert!(task.reply_to.is_some());

        let task_reply_to = task.reply_to.unwrap();
        assert_eq!(task_reply_to.channel_id, "web");
        assert_eq!(task_reply_to.target, "conn-123");

        // Check payload
        let prompt = task.payload.get("prompt").and_then(|v| v.as_str());
        assert_eq!(prompt, Some("Hello, agent!"));

        let session_id = task.payload.get("session_id").and_then(|v| v.as_str());
        assert_eq!(session_id, Some("conn-123"));
    }

    #[test]
    fn test_create_task_with_metadata() {
        let reply_to = ReplyAddress::new("telegram", "chat-456");
        let mut metadata = HashMap::new();
        metadata.insert("user_name".to_string(), serde_json::json!("John"));

        let msg = InboundMessage {
            id: "msg-2".to_string(),
            content: "Test message".to_string(),
            reply_to,
            timestamp: chrono::Utc::now(),
            metadata,
            attachments: Vec::new(),
        };

        let task = create_task_from_message(msg);

        let meta = task.payload.get("metadata").unwrap();
        let user_name = meta.get("user_name").and_then(|v| v.as_str());
        assert_eq!(user_name, Some("John"));
    }

    #[test]
    fn test_task_source() {
        let reply_to = ReplyAddress::new("wechat", "user-789");
        let msg = InboundMessage::new("msg-3", "Hi", reply_to);

        let task = create_task_from_message(msg);

        assert!(matches!(task.source, TaskSource::Custom(ref s) if s == "channel:wechat"));
    }

    #[test]
    fn test_make_reply_address() {
        let addr = make_reply_address("web", "conn-abc");
        assert_eq!(addr.channel_id, "web");
        assert_eq!(addr.target, "conn-abc");
        assert!(addr.thread_id.is_none());
    }

    #[test]
    fn test_channel_bridge_config_default() {
        let config = ChannelBridgeConfig::default();
        assert_eq!(config.default_priority, TaskPriority::Normal);
        assert_eq!(config.task_type, "agent:execute");
    }
