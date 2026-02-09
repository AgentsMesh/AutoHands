//! Integration test for Ark provider.
//!
//! Run with: ARK_API_KEY=your-api-key cargo run --example test_ark
//! Or: cargo run --example test_ark (uses hardcoded test key)

use autohands_protocols::provider::{CompletionRequest, LLMProvider};
use autohands_protocols::types::Message;

#[tokio::main]
async fn main() {
    // Get API key from environment or use the provided test key
    let api_key = std::env::var("ARK_API_KEY")
        .unwrap_or_else(|_| "cb47a61c-751b-45ed-821f-309268aa8485".to_string());

    // Use the model specified by user
    let model = "doubao-seed-1-8-251228";

    println!("Testing Ark provider");
    println!("Model: {}", model);
    println!("---");

    // Create the provider
    let provider = autohands_provider_ark::ArkProvider::new(api_key);

    // Create a simple request
    let request = CompletionRequest::new(
        model.to_string(),
        vec![Message::user("你好！请用一句话介绍一下你自己。")],
    );

    // Make the request
    println!("Sending request...");
    match provider.complete(request).await {
        Ok(response) => {
            println!("✅ Success!");
            println!("Model: {}", response.model);
            println!("Response: {}", response.message.content.text());
            println!("Usage: {} prompt tokens, {} completion tokens",
                response.usage.prompt_tokens,
                response.usage.completion_tokens
            );
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }
}
