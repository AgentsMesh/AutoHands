//! Gemini API types.

use serde::{Deserialize, Serialize};

/// Gemini content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
    FunctionCall { function_call: FunctionCall },
    FunctionResponse { function_response: FunctionResponse },
}

/// Inline data (for images, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineData {
    pub mime_type: String,
    pub data: String,
}

/// Function call from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Function response to send back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Content in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

/// Generation configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub stop_sequences: Vec<String>,
}

/// Tool definition for Gemini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Function declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Generate content request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
}

/// Generate content response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub usage_metadata: Option<UsageMetadata>,
}

/// Candidate response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

/// Safety rating.
#[derive(Debug, Clone, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

/// Usage metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    #[serde(default)]
    pub prompt_token_count: u32,
    #[serde(default)]
    pub candidates_token_count: u32,
    #[serde(default)]
    pub total_token_count: u32,
}

/// Stream response chunk.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub candidates: Option<Vec<Candidate>>,
    pub usage_metadata: Option<UsageMetadata>,
}

/// Error response from API.
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiError {
    pub error: GeminiErrorDetail,
}

/// Error detail.
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiErrorDetail {
    pub code: i32,
    pub message: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_text() {
        let part = Part::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_part_inline_data() {
        let part = Part::InlineData {
            inline_data: InlineData {
                mime_type: "image/png".to_string(),
                data: "base64data".to_string(),
            },
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["inline_data"]["mime_type"], "image/png");
        assert_eq!(json["inline_data"]["data"], "base64data");
    }

    #[test]
    fn test_part_function_call() {
        let part = Part::FunctionCall {
            function_call: FunctionCall {
                name: "get_weather".to_string(),
                args: serde_json::json!({"city": "NYC"}),
            },
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["function_call"]["name"], "get_weather");
        assert_eq!(json["function_call"]["args"]["city"], "NYC");
    }

    #[test]
    fn test_part_function_response() {
        let part = Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "get_weather".to_string(),
                response: serde_json::json!({"temp": 72}),
            },
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["function_response"]["name"], "get_weather");
        assert_eq!(json["function_response"]["response"]["temp"], 72);
    }

    #[test]
    fn test_generation_config_default() {
        let config = GenerationConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_generation_config_with_values() {
        let config = GenerationConfig {
            temperature: Some(0.5),
            top_p: Some(0.9),
            top_k: Some(40),
            max_output_tokens: Some(1024),
            stop_sequences: vec!["END".to_string()],
        };
        let json = serde_json::to_value(&config).unwrap();
        assert!(json["temperature"].as_f64().unwrap() > 0.4);
        assert!(json["topP"].as_f64().unwrap() > 0.8);
        assert_eq!(json["topK"], 40);
        assert_eq!(json["maxOutputTokens"], 1024);
        assert_eq!(json["stopSequences"][0], "END");
    }

    #[test]
    fn test_content_serialization() {
        let content = Content {
            role: "user".to_string(),
            parts: vec![Part::Text {
                text: "Hello".to_string(),
            }],
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_gemini_tool_serialization() {
        let tool = GeminiTool {
            function_declarations: vec![FunctionDeclaration {
                name: "search".to_string(),
                description: "Search for information".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }),
            }],
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["function_declarations"][0]["name"], "search");
    }

    #[test]
    fn test_generate_content_request() {
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "Hello".to_string() }],
            }],
            system_instruction: None,
            generation_config: Some(GenerationConfig::default()),
            tools: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert!(json["contents"].is_array());
        assert_eq!(json["contents"][0]["role"], "user");
    }

    #[test]
    fn test_generate_content_request_with_system() {
        let request = GenerateContentRequest {
            contents: vec![],
            system_instruction: Some(Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "You are helpful".to_string() }],
            }),
            generation_config: None,
            tools: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert!(json["systemInstruction"].is_object());
    }

    #[test]
    fn test_generate_content_response_deserialization() {
        let json = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello!"}]
                },
                "finishReason": "STOP",
                "safetyRatings": []
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        });

        let response: GenerateContentResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].finish_reason, Some("STOP".to_string()));
        assert!(response.usage_metadata.is_some());
        assert_eq!(response.usage_metadata.unwrap().total_token_count, 15);
    }

    #[test]
    fn test_candidate_deserialization() {
        let json = serde_json::json!({
            "content": {
                "role": "model",
                "parts": [{"text": "Response text"}]
            },
            "finishReason": "MAX_TOKENS",
            "safetyRatings": [{
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
            }]
        });

        let candidate: Candidate = serde_json::from_value(json).unwrap();
        assert_eq!(candidate.finish_reason, Some("MAX_TOKENS".to_string()));
        assert_eq!(candidate.safety_ratings.len(), 1);
        assert_eq!(candidate.safety_ratings[0].category, "HARM_CATEGORY_HATE_SPEECH");
    }

    #[test]
    fn test_stream_chunk_deserialization() {
        let json = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": null,
                "safetyRatings": []
            }],
            "usageMetadata": null
        });

        let chunk: StreamChunk = serde_json::from_value(json).unwrap();
        assert!(chunk.candidates.is_some());
        assert!(chunk.usage_metadata.is_none());
    }

    #[test]
    fn test_gemini_error_deserialization() {
        let json = serde_json::json!({
            "error": {
                "code": 429,
                "message": "Rate limit exceeded",
                "status": "RESOURCE_EXHAUSTED"
            }
        });

        let error: GeminiError = serde_json::from_value(json).unwrap();
        assert_eq!(error.error.code, 429);
        assert_eq!(error.error.status, "RESOURCE_EXHAUSTED");
    }

    #[test]
    fn test_part_roundtrip_text() {
        let original = Part::Text { text: "Hello world".to_string() };
        let json = serde_json::to_value(&original).unwrap();
        let parsed: Part = serde_json::from_value(json).unwrap();
        match parsed {
            Part::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected Text part"),
        }
    }

    #[test]
    fn test_part_roundtrip_function_call() {
        let original = Part::FunctionCall {
            function_call: FunctionCall {
                name: "test_func".to_string(),
                args: serde_json::json!({"arg1": "value1"}),
            },
        };
        let json = serde_json::to_value(&original).unwrap();
        let parsed: Part = serde_json::from_value(json).unwrap();
        match parsed {
            Part::FunctionCall { function_call } => {
                assert_eq!(function_call.name, "test_func");
            }
            _ => panic!("Expected FunctionCall part"),
        }
    }

    #[test]
    fn test_usage_metadata_defaults() {
        let json = serde_json::json!({});
        let usage: UsageMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(usage.prompt_token_count, 0);
        assert_eq!(usage.candidates_token_count, 0);
        assert_eq!(usage.total_token_count, 0);
    }
}
