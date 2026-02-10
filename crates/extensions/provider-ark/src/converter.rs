//! Message and tool conversion for Ark API.

use autohands_protocols::provider::CompletionRequest;
use autohands_protocols::types::{
    ContentPart as ProtoContentPart, ImageSource, Message, MessageContent, MessageRole,
};
use autohands_protocols::ToolDefinition;

use crate::api::{
    ApiMessage, ApiTool, ContentPart, FunctionDef, ImageUrl,
    MessageContent as ApiMessageContent,
};

/// Convert protocol messages to Ark API format.
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
        let tool_calls = msg
            .tool_calls
            .iter()
            .map(|tc| crate::api::ToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: crate::api::FunctionCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments.to_string(),
                },
            })
            .collect();

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
    parts
        .iter()
        .filter_map(|part| match part {
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
        })
        .collect()
}

/// Convert tool definitions for Ark API.
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
#[path = "converter_tests.rs"]
mod tests;
