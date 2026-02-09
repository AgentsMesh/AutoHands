//! OpenAI model definitions.

use autohands_protocols::provider::ModelDefinition;

/// Get all available OpenAI models.
pub fn get_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            description: Some("Most capable multimodal model".to_string()),
            context_length: 128_000,
            max_output_tokens: 16384,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(2.5),
            output_cost_per_million: Some(10.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o Mini".to_string(),
            description: Some("Affordable small model for fast tasks".to_string()),
            context_length: 128_000,
            max_output_tokens: 16384,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(0.15),
            output_cost_per_million: Some(0.6),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "gpt-4-turbo".to_string(),
            name: "GPT-4 Turbo".to_string(),
            description: Some("Previous generation flagship model".to_string()),
            context_length: 128_000,
            max_output_tokens: 4096,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(10.0),
            output_cost_per_million: Some(30.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "o1".to_string(),
            name: "o1".to_string(),
            description: Some("Reasoning model for complex tasks".to_string()),
            context_length: 200_000,
            max_output_tokens: 100_000,
            supports_vision: true,
            supports_tools: true,
            supports_system: false,
            input_cost_per_million: Some(15.0),
            output_cost_per_million: Some(60.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "o1-mini".to_string(),
            name: "o1 Mini".to_string(),
            description: Some("Faster reasoning model".to_string()),
            context_length: 128_000,
            max_output_tokens: 65536,
            supports_vision: false,
            supports_tools: true,
            supports_system: false,
            input_cost_per_million: Some(3.0),
            output_cost_per_million: Some(12.0),
            metadata: Default::default(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_not_empty() {
        let models = get_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_gpt4o_capabilities() {
        let models = get_models();
        let gpt4o = models.iter().find(|m| m.id == "gpt-4o").unwrap();
        assert!(gpt4o.supports_vision);
        assert!(gpt4o.supports_tools);
        assert!(gpt4o.supports_system);
    }

    #[test]
    fn test_o1_no_system_prompt() {
        let models = get_models();
        let o1 = models.iter().find(|m| m.id == "o1").unwrap();
        assert!(!o1.supports_system);
    }
}
