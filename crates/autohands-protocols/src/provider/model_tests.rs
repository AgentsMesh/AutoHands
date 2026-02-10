use super::*;

#[test]
fn test_model_definition_new() {
    let model = ModelDefinition::new("gpt-4", "GPT-4");
    assert_eq!(model.id, "gpt-4");
    assert_eq!(model.name, "GPT-4");
    assert_eq!(model.context_length, 128_000);
    assert_eq!(model.max_output_tokens, 4096);
    assert!(!model.supports_vision);
    assert!(model.supports_tools);
    assert!(model.supports_system);
}

#[test]
fn test_model_definition_with_context_length() {
    let model = ModelDefinition::new("test", "Test")
        .with_context_length(200_000);
    assert_eq!(model.context_length, 200_000);
}

#[test]
fn test_model_definition_with_vision() {
    let model = ModelDefinition::new("test", "Test")
        .with_vision();
    assert!(model.supports_vision);
}

#[test]
fn test_model_definition_full() {
    let model = ModelDefinition {
        id: "claude-3-5-sonnet".to_string(),
        name: "Claude 3.5 Sonnet".to_string(),
        description: Some("A powerful model".to_string()),
        context_length: 200_000,
        max_output_tokens: 8192,
        supports_vision: true,
        supports_tools: true,
        supports_system: true,
        input_cost_per_million: Some(3.0),
        output_cost_per_million: Some(15.0),
        metadata: HashMap::new(),
    };
    assert!(model.description.is_some());
    assert!(model.input_cost_per_million.is_some());
}

#[test]
fn test_model_definition_serialization() {
    let model = ModelDefinition::new("test", "Test");
    let json = serde_json::to_string(&model).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("Test"));
}

#[test]
fn test_model_definition_clone() {
    let model = ModelDefinition::new("test", "Test");
    let cloned = model.clone();
    assert_eq!(cloned.id, model.id);
    assert_eq!(cloned.name, model.name);
}

#[test]
fn test_provider_capabilities_default() {
    let caps = ProviderCapabilities::default();
    assert!(!caps.streaming);
    assert!(!caps.tool_calling);
    assert!(!caps.vision);
    assert!(!caps.json_mode);
    assert!(!caps.prompt_caching);
    assert!(!caps.batching);
    assert!(caps.max_concurrent.is_none());
}

#[test]
fn test_provider_capabilities_full() {
    let caps = ProviderCapabilities {
        streaming: true,
        tool_calling: true,
        vision: true,
        json_mode: true,
        prompt_caching: true,
        batching: true,
        max_concurrent: Some(10),
    };
    assert!(caps.streaming);
    assert!(caps.tool_calling);
    assert_eq!(caps.max_concurrent, Some(10));
}

#[test]
fn test_provider_capabilities_serialization() {
    let caps = ProviderCapabilities::default();
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("streaming"));
    assert!(json.contains("false"));
}

#[test]
fn test_provider_capabilities_clone() {
    let caps = ProviderCapabilities {
        streaming: true,
        ..Default::default()
    };
    let cloned = caps.clone();
    assert_eq!(cloned.streaming, caps.streaming);
}

#[test]
fn test_default_true() {
    assert!(default_true());
}

#[test]
fn test_model_definition_builder_chain() {
    let model = ModelDefinition::new("test", "Test")
        .with_context_length(100_000)
        .with_vision();
    assert_eq!(model.context_length, 100_000);
    assert!(model.supports_vision);
}
