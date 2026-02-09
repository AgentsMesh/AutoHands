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
mod tests {
    use super::*;
    use autohands_protocols::types::ToolCall;

    #[test]
    fn test_provider_creation() {
        let provider = GeminiProvider::new("test-key".to_string());
        assert_eq!(provider.id(), "gemini");
        assert!(!provider.models().is_empty());
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.tool_calling);
        assert!(caps.vision);
    }

    #[test]
    fn test_convert_messages() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
    }

    #[test]
    fn test_convert_system() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
        ];

        let system = provider.convert_system(&messages);
        assert!(system.is_some());
    }

    #[test]
    fn test_convert_messages_filters_system() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::system("System prompt"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let contents = provider.convert_messages(&messages);
        // System message should be filtered out
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
    }

    #[test]
    fn test_convert_system_not_found() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::user("Hello"),
        ];

        let system = provider.convert_system(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_tools_empty() {
        let provider = GeminiProvider::new("test-key".to_string());
        let request = CompletionRequest::new("gemini-1.5-flash".to_string(), vec![]);

        let tools = provider.convert_tools(&request);
        assert!(tools.is_none());
    }

    #[test]
    fn test_provider_models_count() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();
        assert_eq!(models.len(), 3);
    }

    #[test]
    fn test_provider_capabilities_detail() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.json_mode);
        assert!(!caps.prompt_caching);
        assert!(!caps.batching);
        assert_eq!(caps.max_concurrent, Some(10));
    }

    #[test]
    fn test_convert_messages_tool_response() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::tool("tool_call_1", "Tool result"),
        ];

        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "user");
    }

    #[test]
    fn test_convert_messages_with_tool_calls() {
        let provider = GeminiProvider::new("test-key".to_string());
        let mut assistant_msg = Message::assistant("");
        assistant_msg.tool_calls = vec![ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"city": "NYC"}),
        }];

        let messages = vec![assistant_msg];
        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "model");
    }

    #[test]
    fn test_models_have_vision() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();
        // gemini-1.5-pro and gemini-1.5-flash should have vision
        let vision_models = models.iter().filter(|m| m.supports_vision).count();
        assert_eq!(vision_models, 2);
    }

    #[test]
    fn test_provider_id_is_gemini() {
        let provider = GeminiProvider::new("key".to_string());
        assert_eq!(provider.id(), "gemini");
    }

    #[test]
    fn test_convert_tools_with_tools() {
        use autohands_protocols::tool::ToolDefinition;

        let provider = GeminiProvider::new("test-key".to_string());
        let tools = vec![
            ToolDefinition::new("read_file", "Read File", "Read a file from disk")
                .with_parameters_schema(serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}}
                })),
        ];
        let request = CompletionRequest::new("gemini-1.5-flash".to_string(), vec![])
            .with_tools(tools);

        let converted = provider.convert_tools(&request);
        assert!(converted.is_some());
        let gemini_tools = converted.unwrap();
        assert_eq!(gemini_tools.len(), 1);
        assert_eq!(gemini_tools[0].function_declarations.len(), 1);
        assert_eq!(gemini_tools[0].function_declarations[0].name, "Read File");
    }

    #[test]
    fn test_convert_response_basic() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "Hello!".to_string() }],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 10,
                candidates_token_count: 5,
                total_token_count: 15,
            }),
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert!(result.message.content.text().contains("Hello!"));
        assert_eq!(result.stop_reason, StopReason::EndTurn);
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 5);
    }

    #[test]
    fn test_convert_response_max_tokens() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "Truncated...".to_string() }],
                },
                finish_reason: Some("MAX_TOKENS".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn test_convert_response_stop_sequence() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: Some("STOP_SEQUENCE".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::StopSequence);
    }

    #[test]
    fn test_convert_response_with_function_call() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::FunctionCall {
                        function_call: FunctionCall {
                            name: "get_weather".to_string(),
                            args: serde_json::json!({"city": "NYC"}),
                        },
                    }],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.message.tool_calls.len(), 1);
        assert_eq!(result.message.tool_calls[0].name, "get_weather");
    }

    #[test]
    fn test_convert_response_unknown_finish_reason() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: Some("UNKNOWN_REASON".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        // Unknown reason defaults to EndTurn
        assert_eq!(result.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn test_convert_response_no_finish_reason() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn test_convert_response_mixed_parts() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![
                        Part::Text { text: "Here's the weather: ".to_string() },
                        Part::FunctionCall {
                            function_call: FunctionCall {
                                name: "get_weather".to_string(),
                                args: serde_json::json!({}),
                            },
                        },
                    ],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert!(result.message.content.text().contains("Here's the weather"));
        assert_eq!(result.message.tool_calls.len(), 1);
    }

    #[test]
    fn test_model_context_lengths() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();

        // Verify context lengths are set correctly
        for model in models {
            if model.id == "gemini-1.5-pro" {
                assert_eq!(model.context_length, 2_000_000);
            } else if model.id == "gemini-2.0-flash" || model.id == "gemini-1.5-flash" {
                assert_eq!(model.context_length, 1_000_000);
            }
        }
    }
}
