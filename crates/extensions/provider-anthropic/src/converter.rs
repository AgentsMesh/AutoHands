//! Message and tool conversion utilities.

use autohands_protocols::provider::CompletionRequest;
use autohands_protocols::types::{Message, MessageRole};

use crate::api::{ApiContent, ApiMessage, ApiTool, ContentBlock};

/// Convert messages to Anthropic API format.
pub fn convert_messages(messages: &[Message]) -> Vec<ApiMessage> {
    messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .map(|m| ApiMessage {
            role: match m.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "user".to_string(),
                MessageRole::System => "user".to_string(),
            },
            content: convert_content(m),
        })
        .collect()
}

/// Convert a single message's content.
pub fn convert_content(message: &Message) -> ApiContent {
    if message.role == MessageRole::Tool {
        if let Some(ref tool_call_id) = message.tool_call_id {
            return ApiContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: tool_call_id.clone(),
                content: message.content.text(),
            }]);
        }
    }

    if !message.tool_calls.is_empty() {
        let mut blocks: Vec<ContentBlock> = vec![];
        let text = message.content.text();
        if !text.is_empty() {
            blocks.push(ContentBlock::Text { text });
        }
        for tc in &message.tool_calls {
            blocks.push(ContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.arguments.clone(),
            });
        }
        return ApiContent::Blocks(blocks);
    }

    ApiContent::Text(message.content.text())
}

/// Convert tools to Anthropic API format.
pub fn convert_tools(request: &CompletionRequest) -> Vec<ApiTool> {
    request
        .tools
        .iter()
        .map(|t| ApiTool {
            name: t.id.clone(),
            description: t.description.clone(),
            input_schema: t
                .parameters_schema
                .clone()
                .unwrap_or(serde_json::json!({"type": "object"})),
        })
        .collect()
}

#[cfg(test)]
#[path = "converter_tests.rs"]
mod tests;
