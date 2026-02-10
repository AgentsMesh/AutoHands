//! Gemini LLM provider implementation.

use std::collections::HashMap;

use async_trait::async_trait;
use futures::StreamExt;
use tracing::debug;

use autohands_protocols::error::ProviderError;
use autohands_protocols::provider::{
    ChunkType, CompletionChunk, CompletionRequest, CompletionResponse, CompletionStream,
    LLMProvider, ModelDefinition, ProviderCapabilities,
};
use autohands_protocols::types::{Message, MessageRole, StopReason, Usage};

use crate::client::GeminiClient;
use crate::types::*;

/// Gemini LLM provider.
pub struct GeminiProvider {
    client: GeminiClient,
    models: Vec<ModelDefinition>,
    capabilities: ProviderCapabilities,
}

impl GeminiProvider {
    /// Create a new Gemini provider.
    pub fn new(api_key: String) -> Self {
        Self {
            client: GeminiClient::new(api_key),
            models: vec![
                ModelDefinition::new("gemini-2.0-flash", "Gemini 2.0 Flash")
                    .with_context_length(1_000_000),
                ModelDefinition::new("gemini-1.5-pro", "Gemini 1.5 Pro")
                    .with_context_length(2_000_000)
                    .with_vision(),
                ModelDefinition::new("gemini-1.5-flash", "Gemini 1.5 Flash")
                    .with_context_length(1_000_000)
                    .with_vision(),
            ],
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: true,
                json_mode: true,
                prompt_caching: false,
                batching: false,
                max_concurrent: Some(10),
            },
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<Content> {
        messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User | MessageRole::Tool => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::System => "user",
                };

                let parts = if msg.role == MessageRole::Tool {
                    vec![Part::FunctionResponse {
                        function_response: FunctionResponse {
                            name: msg.tool_call_id.clone().unwrap_or_default(),
                            response: serde_json::json!({ "result": msg.content.text() }),
                        },
                    }]
                } else if !msg.tool_calls.is_empty() {
                    msg.tool_calls
                        .iter()
                        .map(|tc| Part::FunctionCall {
                            function_call: FunctionCall {
                                name: tc.name.clone(),
                                args: tc.arguments.clone(),
                            },
                        })
                        .collect()
                } else {
                    vec![Part::Text {
                        text: msg.content.text(),
                    }]
                };

                Content {
                    role: role.to_string(),
                    parts,
                }
            })
            .collect()
    }

    fn convert_system(&self, messages: &[Message]) -> Option<Content> {
        messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .map(|msg| Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: msg.content.text(),
                }],
            })
    }

    fn convert_tools(&self, request: &CompletionRequest) -> Option<Vec<GeminiTool>> {
        if request.tools.is_empty() {
            return None;
        }

        Some(vec![GeminiTool {
            function_declarations: request
                .tools
                .iter()
                .map(|tool| FunctionDeclaration {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.parameters_schema.clone().unwrap_or_default(),
                })
                .collect(),
        }])
    }

    fn convert_response(&self, response: GenerateContentResponse, model: &str) -> CompletionResponse {
        let candidate = &response.candidates[0];
        let content = &candidate.content;

        // Extract text and tool calls
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for part in &content.parts {
            match part {
                Part::Text { text } => text_parts.push(text.clone()),
                Part::FunctionCall { function_call } => {
                    tool_calls.push(autohands_protocols::types::ToolCall {
                        id: format!("call_{}", uuid::Uuid::new_v4()),
                        name: function_call.name.clone(),
                        arguments: function_call.args.clone(),
                    });
                }
                _ => {}
            }
        }

        let message = if tool_calls.is_empty() {
            Message::assistant(text_parts.join(""))
        } else {
            let mut msg = Message::assistant(text_parts.join(""));
            msg.tool_calls = tool_calls;
            msg
        };

        let stop_reason = match candidate.finish_reason.as_deref() {
            Some("STOP") => StopReason::EndTurn,
            Some("MAX_TOKENS") => StopReason::MaxTokens,
            Some("STOP_SEQUENCE") => StopReason::StopSequence,
            _ => StopReason::EndTurn,
        };

        let usage = response.usage_metadata.map(|u| Usage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
            ..Default::default()
        }).unwrap_or_default();

        CompletionResponse {
            id: format!("gemini-{}", uuid::Uuid::new_v4()),
            model: model.to_string(),
            message,
            stop_reason,
            usage,
            metadata: HashMap::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn models(&self) -> &[ModelDefinition] {
        &self.models
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        debug!("Gemini complete: model={}", request.model);

        let gemini_request = GenerateContentRequest {
            contents: self.convert_messages(&request.messages),
            system_instruction: self.convert_system(&request.messages),
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                top_p: request.top_p,
                max_output_tokens: request.max_tokens,
                stop_sequences: request.stop.clone(),
                ..Default::default()
            }),
            tools: self.convert_tools(&request),
        };

        let response = self.client.generate_content(&request.model, gemini_request).await?;
        Ok(self.convert_response(response, &request.model))
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, ProviderError> {
        debug!("Gemini stream: model={}", request.model);

        let gemini_request = GenerateContentRequest {
            contents: self.convert_messages(&request.messages),
            system_instruction: self.convert_system(&request.messages),
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                top_p: request.top_p,
                max_output_tokens: request.max_tokens,
                stop_sequences: request.stop.clone(),
                ..Default::default()
            }),
            tools: self.convert_tools(&request),
        };

        let stream = self.client.generate_content_stream(&request.model, gemini_request).await?;

        let mapped_stream = stream.map(|result| {
            result.map(|chunk| {
                let mut completion_chunk = CompletionChunk {
                    chunk_type: ChunkType::ContentDelta,
                    delta: None,
                    tool_call: None,
                    stop_reason: None,
                    usage: None,
                };

                if let Some(candidates) = chunk.candidates {
                    if let Some(candidate) = candidates.first() {
                        for part in &candidate.content.parts {
                            if let Part::Text { text } = part {
                                completion_chunk.delta = Some(text.clone());
                            }
                        }

                        if let Some(reason) = &candidate.finish_reason {
                            completion_chunk.stop_reason = Some(match reason.as_str() {
                                "STOP" => StopReason::EndTurn,
                                "MAX_TOKENS" => StopReason::MaxTokens,
                                _ => StopReason::EndTurn,
                            });
                        }
                    }
                }

                if let Some(usage) = chunk.usage_metadata {
                    completion_chunk.usage = Some(Usage {
                        prompt_tokens: usage.prompt_token_count,
                        completion_tokens: usage.candidates_token_count,
                        total_tokens: usage.total_token_count,
                        ..Default::default()
                    });
                }

                completion_chunk
            })
        });

        Ok(Box::pin(mapped_stream))
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
