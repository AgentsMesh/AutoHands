//! Gemini API client.

use std::pin::Pin;

use futures::Stream;
use reqwest::Client;
use tracing::debug;

use autohands_protocols::error::ProviderError;

use crate::types::*;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Gemini API client.
pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    /// Create a new Gemini client.
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Generate content (non-streaming).
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, ProviderError> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            BASE_URL, model, self.api_key
        );

        debug!("Gemini generate_content: model={}", model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !status.is_success() {
            let error: Result<GeminiError, _> = serde_json::from_str(&body);
            return match error {
                Ok(e) => Err(ProviderError::ApiError {
                    status: status.as_u16(),
                    message: e.error.message,
                }),
                Err(_) => Err(ProviderError::ApiError {
                    status: status.as_u16(),
                    message: body,
                }),
            };
        }

        serde_json::from_str(&body).map_err(|e| {
            ProviderError::ApiError {
                status: 500,
                message: format!("Failed to parse response: {}", e),
            }
        })
    }

    /// Generate content (streaming).
    pub async fn generate_content_stream(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>
    {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
            BASE_URL, model, self.api_key
        );

        debug!("Gemini stream generate_content: model={}", model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;
            let error: Result<GeminiError, _> = serde_json::from_str(&body);
            return match error {
                Ok(e) => Err(ProviderError::ApiError {
                    status: status.as_u16(),
                    message: e.error.message,
                }),
                Err(_) => Err(ProviderError::ApiError {
                    status: status.as_u16(),
                    message: body,
                }),
            };
        }

        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            use futures::StreamExt;
            let mut buffer = String::new();

            while let Some(chunk) = bytes_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        // Process complete SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            if let Some(data) = event.strip_prefix("data: ") {
                                if data.trim() == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<StreamChunk>(data) {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(e) => yield Err(ProviderError::StreamError(e.to_string())),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::StreamError(e.to_string()));
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test-key".to_string());
        assert_eq!(client.api_key, "test-key");
    }

    #[test]
    fn test_client_creation_with_empty_key() {
        let client = GeminiClient::new(String::new());
        assert!(client.api_key.is_empty());
    }

    #[test]
    fn test_client_creation_with_long_key() {
        let long_key = "k".repeat(1000);
        let client = GeminiClient::new(long_key.clone());
        assert_eq!(client.api_key, long_key);
    }

    #[test]
    fn test_base_url_constant() {
        assert_eq!(BASE_URL, "https://generativelanguage.googleapis.com/v1beta");
    }

    #[test]
    fn test_client_creation_with_special_chars() {
        let key = "AIza-SyC_test_KEY-123=";
        let client = GeminiClient::new(key.to_string());
        assert_eq!(client.api_key, key);
    }

    #[test]
    fn test_generate_content_request_serialization() {
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "Hello".to_string() }],
            }],
            system_instruction: None,
            generation_config: None,
            tools: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_generate_content_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hi there!"}]
                },
                "finishReason": "STOP",
                "safetyRatings": []
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 10,
                "totalTokenCount": 15
            }
        }"#;
        let response: GenerateContentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].finish_reason, Some("STOP".to_string()));
    }

    #[test]
    fn test_gemini_error_deserialization() {
        let json = r#"{
            "error": {
                "code": 400,
                "message": "Invalid request",
                "status": "INVALID_ARGUMENT"
            }
        }"#;
        let error: GeminiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error.code, 400);
        assert_eq!(error.error.message, "Invalid request");
    }

    #[test]
    fn test_stream_chunk_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": null,
                "safetyRatings": []
            }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.candidates.is_some());
        assert!(chunk.usage_metadata.is_none());
    }

    #[test]
    fn test_stream_chunk_with_usage() {
        let json = r#"{
            "candidates": null,
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.candidates.is_none());
        assert!(chunk.usage_metadata.is_some());
        let usage = chunk.usage_metadata.unwrap();
        assert_eq!(usage.total_token_count, 15);
    }

    #[test]
    fn test_content_with_function_call() {
        let content = Content {
            role: "model".to_string(),
            parts: vec![Part::FunctionCall {
                function_call: FunctionCall {
                    name: "get_weather".to_string(),
                    args: serde_json::json!({"city": "NYC"}),
                },
            }],
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("function_call"));
        assert!(json.contains("get_weather"));
    }

    #[test]
    fn test_content_with_function_response() {
        let content = Content {
            role: "user".to_string(),
            parts: vec![Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: "get_weather".to_string(),
                    response: serde_json::json!({"temp": 72, "unit": "F"}),
                },
            }],
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("function_response"));
        assert!(json.contains("get_weather"));
    }

    #[test]
    fn test_generation_config_serialization() {
        let config = GenerationConfig {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            max_output_tokens: Some(1024),
            stop_sequences: vec!["END".to_string()],
        };
        let json = serde_json::to_value(&config).unwrap();
        // Use approximate comparison for floating point
        assert!(json["temperature"].as_f64().unwrap() > 0.69 && json["temperature"].as_f64().unwrap() < 0.71);
        assert_eq!(json["maxOutputTokens"], 1024);
    }

    #[test]
    fn test_gemini_tool_serialization() {
        let tool = GeminiTool {
            function_declarations: vec![FunctionDeclaration {
                name: "search".to_string(),
                description: "Search the web".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }),
            }],
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("function_declarations"));
        assert!(json.contains("search"));
    }

    #[test]
    fn test_safety_rating_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Safe content"}]
                },
                "finishReason": "STOP",
                "safetyRatings": [{
                    "category": "HARM_CATEGORY_HARASSMENT",
                    "probability": "NEGLIGIBLE"
                }]
            }]
        }"#;
        let response: GenerateContentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates[0].safety_ratings.len(), 1);
        assert_eq!(
            response.candidates[0].safety_ratings[0].category,
            "HARM_CATEGORY_HARASSMENT"
        );
    }

    #[test]
    fn test_generate_content_request_with_all_fields() {
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "Hello".to_string() }],
            }],
            system_instruction: Some(Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "Be helpful".to_string() }],
            }),
            generation_config: Some(GenerationConfig {
                temperature: Some(0.5),
                top_p: None,
                top_k: None,
                max_output_tokens: Some(500),
                stop_sequences: vec![],
            }),
            tools: Some(vec![GeminiTool {
                function_declarations: vec![FunctionDeclaration {
                    name: "test".to_string(),
                    description: "Test tool".to_string(),
                    parameters: serde_json::json!({}),
                }],
            }]),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("systemInstruction"));
        assert!(json.contains("generationConfig"));
        assert!(json.contains("tools"));
    }

    #[test]
    fn test_inline_data_part() {
        let part = Part::InlineData {
            inline_data: InlineData {
                mime_type: "image/png".to_string(),
                data: "base64encodeddata".to_string(),
            },
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("inline_data"));
        assert!(json.contains("image/png"));
    }

    #[test]
    fn test_response_without_usage_metadata() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Response"}]
                },
                "finishReason": "STOP",
                "safetyRatings": []
            }]
        }"#;
        let response: GenerateContentResponse = serde_json::from_str(json).unwrap();
        assert!(response.usage_metadata.is_none());
    }

    #[test]
    fn test_candidate_without_finish_reason() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Partial"}]
                },
                "safetyRatings": []
            }]
        }"#;
        let response: GenerateContentResponse = serde_json::from_str(json).unwrap();
        assert!(response.candidates[0].finish_reason.is_none());
    }

    // Wiremock-based tests for actual HTTP calls
    mod http_tests {
        use super::*;
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        #[tokio::test]
        async fn test_generate_content_success() {
            let mock_server = MockServer::start().await;

            let response_body = r#"{
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{"text": "Hello back!"}]
                    },
                    "finishReason": "STOP",
                    "safetyRatings": []
                }],
                "usageMetadata": {
                    "promptTokenCount": 5,
                    "candidatesTokenCount": 3,
                    "totalTokenCount": 8
                }
            }"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:generateContent.*"))
                .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            // Create client with mock server URL
            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![Content {
                    role: "user".to_string(),
                    parts: vec![Part::Text { text: "Hello".to_string() }],
                }],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content("gemini-pro", request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.candidates.len(), 1);
            assert_eq!(response.candidates[0].finish_reason, Some("STOP".to_string()));
        }

        #[tokio::test]
        async fn test_generate_content_api_error() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{
                "error": {
                    "code": 400,
                    "message": "Invalid request",
                    "status": "INVALID_ARGUMENT"
                }
            }"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:generateContent.*"))
                .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content("gemini-pro", request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 400);
                    assert!(message.contains("Invalid request"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_generate_content_non_json_error() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:generateContent.*"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content("gemini-pro", request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 500);
                    assert!(message.contains("Internal Server Error"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_generate_content_invalid_response() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:generateContent.*"))
                .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content("gemini-pro", request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 500);
                    assert!(message.contains("Failed to parse"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_generate_content_stream_success() {
            let mock_server = MockServer::start().await;

            let sse_body = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"safetyRatings\":[]}]}\n\n";

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:streamGenerateContent.*"))
                .respond_with(ResponseTemplate::new(200).set_body_string(sse_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![Content {
                    role: "user".to_string(),
                    parts: vec![Part::Text { text: "Hello".to_string() }],
                }],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content_stream("gemini-pro", request).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_generate_content_stream_api_error() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{"error":{"code":429,"message":"Rate limit","status":"RESOURCE_EXHAUSTED"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:streamGenerateContent.*"))
                .respond_with(ResponseTemplate::new(429).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content_stream("gemini-pro", request).await;
            assert!(result.is_err());
            let err = result.err().unwrap();
            match err {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 429);
                    assert!(message.contains("Rate limit"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_generate_content_stream_non_json_error() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path_regex(r".*/models/.*:streamGenerateContent.*"))
                .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let client = GeminiClientWithBaseUrl::new("test-key".to_string(), mock_server.uri());
            let request = GenerateContentRequest {
                contents: vec![],
                system_instruction: None,
                generation_config: None,
                tools: None,
            };

            let result = client.generate_content_stream("gemini-pro", request).await;
            assert!(result.is_err());
            let err = result.err().unwrap();
            match err {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 503);
                    assert!(message.contains("Service Unavailable"));
                }
                _ => panic!("Expected ApiError"),
            }
        }
    }

    // Test helper client with configurable base URL
    struct GeminiClientWithBaseUrl {
        client: Client,
        api_key: String,
        base_url: String,
    }

    impl GeminiClientWithBaseUrl {
        fn new(api_key: String, base_url: String) -> Self {
            Self {
                client: Client::new(),
                api_key,
                base_url,
            }
        }

        async fn generate_content(
            &self,
            model: &str,
            request: GenerateContentRequest,
        ) -> Result<GenerateContentResponse, ProviderError> {
            let url = format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, model, self.api_key
            );

            let response = self
                .client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;

            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;

            if !status.is_success() {
                let error: Result<GeminiError, _> = serde_json::from_str(&body);
                return match error {
                    Ok(e) => Err(ProviderError::ApiError {
                        status: status.as_u16(),
                        message: e.error.message,
                    }),
                    Err(_) => Err(ProviderError::ApiError {
                        status: status.as_u16(),
                        message: body,
                    }),
                };
            }

            serde_json::from_str(&body).map_err(|e| {
                ProviderError::ApiError {
                    status: 500,
                    message: format!("Failed to parse response: {}", e),
                }
            })
        }

        async fn generate_content_stream(
            &self,
            model: &str,
            request: GenerateContentRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>
        {
            let url = format!(
                "{}/models/{}:streamGenerateContent?key={}&alt=sse",
                self.base_url, model, self.api_key
            );

            let response = self
                .client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let body = response
                    .text()
                    .await
                    .map_err(|e| ProviderError::Network(e.to_string()))?;
                let error: Result<GeminiError, _> = serde_json::from_str(&body);
                return match error {
                    Ok(e) => Err(ProviderError::ApiError {
                        status: status.as_u16(),
                        message: e.error.message,
                    }),
                    Err(_) => Err(ProviderError::ApiError {
                        status: status.as_u16(),
                        message: body,
                    }),
                };
            }

            let stream = async_stream::stream! {
                let mut bytes_stream = response.bytes_stream();
                use futures::StreamExt;
                let mut buffer = String::new();

                while let Some(chunk) = bytes_stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            while let Some(pos) = buffer.find("\n\n") {
                                let event = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                if let Some(data) = event.strip_prefix("data: ") {
                                    if data.trim() == "[DONE]" {
                                        continue;
                                    }
                                    match serde_json::from_str::<StreamChunk>(data) {
                                        Ok(chunk) => yield Ok(chunk),
                                        Err(e) => yield Err(ProviderError::StreamError(e.to_string())),
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            yield Err(ProviderError::StreamError(e.to_string()));
                        }
                    }
                }
            };

            Ok(Box::pin(stream))
        }
    }
}
