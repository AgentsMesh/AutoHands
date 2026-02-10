//! Vision-capable LLM provider wrapper.

use std::collections::HashMap;
use std::sync::Arc;

use autohands_protocols::error::ToolError;
use autohands_protocols::provider::{CompletionRequest, LLMProvider};
use autohands_protocols::types::{
    ContentPart, ImageSource, Message, MessageContent, MessageRole,
};

/// Wrapper for vision-capable LLM provider.
pub struct VisionProvider {
    provider: Arc<dyn LLMProvider>,
    model: String,
}

impl VisionProvider {
    /// Create a new vision provider wrapper.
    pub fn new(provider: Arc<dyn LLMProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }

    /// Analyze an image with a prompt.
    ///
    /// Note: Screenshots are in JPEG format (60% quality) to reduce size.
    pub async fn analyze(&self, image_base64: &str, prompt: &str) -> Result<String, ToolError> {
        let message = Message {
            role: MessageRole::User,
            content: MessageContent::Parts(vec![
                ContentPart::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/jpeg".to_string(),
                        data: image_base64.to_string(),
                    },
                },
                ContentPart::Text {
                    text: prompt.to_string(),
                },
            ]),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
        };

        let request = CompletionRequest::new(&self.model, vec![message])
            .with_max_tokens(1024)
            .with_temperature(0.0);

        let response = self
            .provider
            .complete(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Vision API error: {}", e)))?;

        Ok(response.message.content.text())
    }
}
