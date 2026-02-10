    use super::*;
    use autohands_protocols::provider::CompletionRequest;
    use autohands_protocols::tool::ToolDefinition;
    use autohands_protocols::types::{MessageContent, ToolCall};

    #[test]
    fn test_convert_user_message() {
        let msg = Message::user("Hello");
        let content = convert_content(&msg);
        match content {
            ApiContent::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_convert_messages_filters_system() {
        let messages = vec![
            Message::system("System prompt"),
            Message::user("Hello"),
        ];
        let converted = convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
    }

    #[test]
    fn test_convert_assistant_message() {
        let msg = Message::assistant("I can help you");
        let converted = convert_messages(&[msg]);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "assistant");
    }

    #[test]
    fn test_convert_tool_result_message() {
        let msg = Message {
            role: MessageRole::Tool,
            content: MessageContent::Text("Tool output".to_string()),
            name: None,
            tool_calls: vec![],
            tool_call_id: Some("tool_123".to_string()),
            metadata: Default::default(),
        };

        let content = convert_content(&msg);
        match content {
            ApiContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1);
                match &blocks[0] {
                    ContentBlock::ToolResult { tool_use_id, content } => {
                        assert_eq!(tool_use_id, "tool_123");
                        assert_eq!(content, "Tool output");
                    }
                    _ => panic!("Expected ToolResult block"),
                }
            }
            _ => panic!("Expected blocks content"),
        }
    }

    #[test]
    fn test_convert_message_with_tool_calls() {
        let msg = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text("Let me search".to_string()),
            name: None,
            tool_calls: vec![
                ToolCall {
                    id: "tc_1".to_string(),
                    name: "search".to_string(),
                    arguments: serde_json::json!({"query": "rust"}),
                },
            ],
            tool_call_id: None,
            metadata: Default::default(),
        };

        let content = convert_content(&msg);
        match content {
            ApiContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2); // Text + ToolUse
                match &blocks[0] {
                    ContentBlock::Text { text } => assert_eq!(text, "Let me search"),
                    _ => panic!("Expected Text block first"),
                }
                match &blocks[1] {
                    ContentBlock::ToolUse { id, name, input } => {
                        assert_eq!(id, "tc_1");
                        assert_eq!(name, "search");
                        assert_eq!(input["query"], "rust");
                    }
                    _ => panic!("Expected ToolUse block"),
                }
            }
            _ => panic!("Expected blocks content"),
        }
    }

    #[test]
    fn test_convert_message_with_tool_calls_no_text() {
        let msg = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text("".to_string()),
            name: None,
            tool_calls: vec![
                ToolCall {
                    id: "tc_1".to_string(),
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": "/tmp/file.txt"}),
                },
            ],
            tool_call_id: None,
            metadata: Default::default(),
        };

        let content = convert_content(&msg);
        match content {
            ApiContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1); // Only ToolUse, no empty text
                match &blocks[0] {
                    ContentBlock::ToolUse { id, name, .. } => {
                        assert_eq!(id, "tc_1");
                        assert_eq!(name, "read_file");
                    }
                    _ => panic!("Expected ToolUse block"),
                }
            }
            _ => panic!("Expected blocks content"),
        }
    }

    #[test]
    fn test_convert_multiple_messages() {
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
            Message::user("How are you?"),
        ];

        let converted = convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }

    #[test]
    fn test_convert_tools_empty() {
        let request = CompletionRequest::new("claude-sonnet-4-20250514", vec![]);
        let tools = convert_tools(&request);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_convert_tools_with_schema() {
        let tool_def = ToolDefinition::new("read_file", "Read File", "Read contents of a file")
            .with_parameters_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }));

        let mut request = CompletionRequest::new("claude-sonnet-4-20250514", vec![]);
        request.tools = vec![tool_def];

        let tools = convert_tools(&request);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(tools[0].description, "Read contents of a file");
        assert!(tools[0].input_schema["properties"]["path"].is_object());
    }

    #[test]
    fn test_convert_tools_without_schema() {
        let tool_def = ToolDefinition::new("simple_tool", "Simple", "A simple tool");

        let mut request = CompletionRequest::new("claude-sonnet-4-20250514", vec![]);
        request.tools = vec![tool_def];

        let tools = convert_tools(&request);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "simple_tool");
        // Should have default schema
        assert_eq!(tools[0].input_schema["type"], "object");
    }

    #[test]
    fn test_convert_multiple_tools() {
        let tool1 = ToolDefinition::new("tool1", "Tool 1", "First tool");
        let tool2 = ToolDefinition::new("tool2", "Tool 2", "Second tool");

        let mut request = CompletionRequest::new("claude-sonnet-4-20250514", vec![]);
        request.tools = vec![tool1, tool2];

        let tools = convert_tools(&request);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "tool1");
        assert_eq!(tools[1].name, "tool2");
    }
