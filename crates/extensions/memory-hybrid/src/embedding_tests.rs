use super::*;

#[test]
fn test_config_defaults() {
    let config = OpenAIEmbeddingConfig::new("test-key");
    assert_eq!(config.api_key, "test-key");
    assert_eq!(config.model, "text-embedding-3-small");
    assert_eq!(config.dimension, 1536);
}

#[test]
fn test_config_builder() {
    let config = OpenAIEmbeddingConfig::new("key")
        .with_model("text-embedding-3-large")
        .with_dimension(3072)
        .with_base_url("https://custom.api.com");

    assert_eq!(config.model, "text-embedding-3-large");
    assert_eq!(config.dimension, 3072);
    assert_eq!(config.base_url, "https://custom.api.com");
}

#[test]
fn test_provider_dimension() {
    let provider = OpenAIEmbedding::from_api_key("test-key");
    assert_eq!(provider.dimension(), 1536);
}

#[test]
fn test_config_clone() {
    let config = OpenAIEmbeddingConfig::new("key");
    let cloned = config.clone();
    assert_eq!(cloned.api_key, "key");
}
