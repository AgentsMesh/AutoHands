//! Message and tool conversion for OpenAI API.

use autohands_protocols::provider::CompletionRequest;
use autohands_protocols::types::{ContentPart as ProtoContentPart, ImageSource, Message, MessageContent, MessageRole};
use autohands_protocols::ToolDefinition;

use crate::api::{
    ApiMessage, ApiTool, ContentPart, FunctionDef, ImageUrl,
    MessageContent as ApiMessageContent,
};

/// Convert protocol messages to OpenAI API format.
pub fn convert_messages(messages: &[Message]) -> Vec<ApiMessage> {
    messages.iter().map(convert_message).collect()
}

fn convert_message(msg: &Message) -> ApiMessage {
    let role = match msg.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    };

    // Handle tool calls from assistant messages
    if !msg.tool_calls.is_empty() {
        let tool_calls = msg.tool_calls.iter().map(|tc| crate::api::ToolCall {
            id: tc.id.clone(),
            call_type: "function".to_string(),
            function: crate::api::FunctionCall {
                name: tc.name.clone(),
                arguments: tc.arguments.to_string(),
            },
        }).collect();

        return ApiMessage {
            role: role.to_string(),
            content: Some(ApiMessageContent::Text(msg.content.text())),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        };
    }

    // Handle tool response messages
    if msg.role == MessageRole::Tool {
        return ApiMessage {
            role: role.to_string(),
            content: Some(ApiMessageContent::Text(msg.content.text())),
            tool_calls: None,
            tool_call_id: msg.tool_call_id.clone(),
            name: None,
        };
    }

    // Handle regular messages
    match &msg.content {
        MessageContent::Text(text) => ApiMessage {
            role: role.to_string(),
            content: Some(ApiMessageContent::Text(text.clone())),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        MessageContent::Parts(parts) => {
            let api_parts = convert_parts(parts);
            ApiMessage {
                role: role.to_string(),
                content: Some(ApiMessageContent::Parts(api_parts)),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }
        }
    }
}

fn convert_parts(parts: &[ProtoContentPart]) -> Vec<ContentPart> {
    parts.iter().filter_map(|part| match part {
        ProtoContentPart::Text { text } => Some(ContentPart::Text { text: text.clone() }),
        ProtoContentPart::Image { source } => {
            let url = match source {
                ImageSource::Base64 { media_type, data } => {
                    format!("data:{};base64,{}", media_type, data)
                }
                ImageSource::Url { url } => url.clone(),
            };
            Some(ContentPart::ImageUrl {
                image_url: ImageUrl { url, detail: None },
            })
        }
        // Skip tool use/result parts as they're handled differently
        ProtoContentPart::ToolUse { .. } | ProtoContentPart::ToolResult { .. } => None,
    }).collect()
}

/// Convert tool definitions for OpenAI API.
pub fn convert_tools(request: &CompletionRequest) -> Vec<ApiTool> {
    request.tools.iter().map(convert_tool).collect()
}

fn convert_tool(tool: &ToolDefinition) -> ApiTool {
    let params = tool.parameters_schema.clone().unwrap_or_else(|| {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    });

    ApiTool {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: tool.id.clone(),
            description: tool.description.clone(),
            parameters: params,
        },
    }
}

#[cfg(test)]
mod tests {
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
            arguments: serde_json::json!({"city": "NYC"}),
        }];
        let api_msg = convert_message(&msg);
        assert!(api_msg.tool_calls.is_some());
        let tool_calls = api_msg.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].function.name, "get_weather");
    }

    #[test]
    fn test_convert_multipart_message() {
        let msg = Message {
            role: MessageRole::User,
            content: MessageContent::Parts(vec![
                ProtoContentPart::Text { text: "Hello".to_string() },
            ]),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
            metadata: Default::default(),
        };
        let api_msg = convert_message(&msg);
        assert!(matches!(api_msg.content, Some(ApiMessageContent::Parts(_))));
    }

    #[test]
    fn test_convert_image_part_url() {
        let parts = vec![ProtoContentPart::Image {
            source: ImageSource::Url {
                url: "https://example.com/image.png".to_string(),
            },
        }];
        let result = convert_parts(&parts);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ContentPart::ImageUrl { image_url } => {
                assert_eq!(image_url.url, "https://example.com/image.png");
            }
            _ => panic!("Expected ImageUrl"),
        }
    }

    #[test]
    fn test_convert_image_part_base64() {
        let parts = vec![ProtoContentPart::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "abc123".to_string(),
            },
        }];
        let result = convert_parts(&parts);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ContentPart::ImageUrl { image_url } => {
                assert!(image_url.url.starts_with("data:image/png;base64,"));
            }
            _ => panic!("Expected ImageUrl"),
        }
    }

    #[test]
    fn test_convert_tools() {
        let request = CompletionRequest::new("gpt-4o", vec![])
            .with_tools(vec![ToolDefinition::new("test_tool", "Test Tool", "A test")]);
        let tools = convert_tools(&request);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "test_tool");
    }

    #[test]
    fn test_convert_tool_with_schema() {
        let mut tool = ToolDefinition::new("test", "Test", "desc");
        tool.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "arg": { "type": "string" }
            }
        }));
        let api_tool = convert_tool(&tool);
        assert!(api_tool.function.parameters.get("properties").is_some());
    }
}
