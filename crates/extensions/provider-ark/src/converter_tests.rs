    use super::*;
    use autohands_protocols::types::ToolCall;

    #[test]
    fn test_convert_text_message() {
        let msg = Message::user("Hello");
        let api_msg = convert_message(&msg);
        assert_eq!(api_msg.role, "user");
        assert!(matches!(api_msg.content, Some(ApiMessageContent::Text(_))));
    }

    #[test]
    fn test_convert_assistant_message() {
        let msg = Message::assistant("Hello back");
        let api_msg = convert_message(&msg);
        assert_eq!(api_msg.role, "assistant");
    }

    #[test]
    fn test_convert_system_message() {
        let msg = Message::system("You are helpful");
        let api_msg = convert_message(&msg);
        assert_eq!(api_msg.role, "system");
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            Message::system("System prompt"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];
        let result = convert_messages(&messages);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn test_convert_tool_message() {
        let mut msg = Message::user("Tool result");
        msg.role = MessageRole::Tool;
        msg.tool_call_id = Some("call_123".to_string());
        let api_msg = convert_message(&msg);
        assert_eq!(api_msg.role, "tool");
        assert_eq!(api_msg.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_convert_message_with_tool_calls() {
        let mut msg = Message::assistant("Calling tool");
        msg.tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"city": "Beijing"}),
        }];
        let api_msg = convert_message(&msg);
        assert!(api_msg.tool_calls.is_some());
        let tool_calls = api_msg.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].function.name, "get_weather");
    }

    #[test]
    fn test_convert_tools() {
        let request = CompletionRequest::new("doubao-pro-32k", vec![])
            .with_tools(vec![ToolDefinition::new("test_tool", "Test Tool", "A test")]);
        let tools = convert_tools(&request);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "test_tool");
    }
