//! Anthropic model definitions.

use autohands_protocols::provider::ModelDefinition;

/// Get all available Anthropic models.
pub fn get_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "claude-opus-4-5-20251101".to_string(),
            name: "Claude Opus 4.5".to_string(),
            description: Some("Most capable model for complex tasks".to_string()),
            context_length: 200_000,
            max_output_tokens: 8192,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(15.0),
            output_cost_per_million: Some(75.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "claude-sonnet-4-20250514".to_string(),
            name: "Claude Sonnet 4".to_string(),
            description: Some("Balanced performance and cost".to_string()),
            context_length: 200_000,
            max_output_tokens: 8192,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(3.0),
            output_cost_per_million: Some(15.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "claude-haiku-4-20250514".to_string(),
            name: "Claude Haiku 4".to_string(),
            description: Some("Fast and cost-effective".to_string()),
            context_length: 200_000,
            max_output_tokens: 8192,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(0.25),
            output_cost_per_million: Some(1.25),
            metadata: Default::default(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_models_returns_three_models() {
        let models = get_models();
        assert_eq!(models.len(), 3);
    }

    #[test]
    fn test_opus_model() {
        let models = get_models();
        let opus = models.iter().find(|m| m.id.contains("opus")).unwrap();

        assert_eq!(opus.name, "Claude Opus 4.5");
        assert_eq!(opus.context_length, 200_000);
        assert_eq!(opus.max_output_tokens, 8192);
        assert!(opus.supports_vision);
        assert!(opus.supports_tools);
        assert!(opus.supports_system);
        assert_eq!(opus.input_cost_per_million, Some(15.0));
        assert_eq!(opus.output_cost_per_million, Some(75.0));
    }

    #[test]
    fn test_sonnet_model() {
        let models = get_models();
        let sonnet = models.iter().find(|m| m.id.contains("sonnet")).unwrap();

        assert_eq!(sonnet.name, "Claude Sonnet 4");
        assert_eq!(sonnet.context_length, 200_000);
        assert_eq!(sonnet.input_cost_per_million, Some(3.0));
        assert_eq!(sonnet.output_cost_per_million, Some(15.0));
    }

    #[test]
    fn test_haiku_model() {
        let models = get_models();
        let haiku = models.iter().find(|m| m.id.contains("haiku")).unwrap();

        assert_eq!(haiku.name, "Claude Haiku 4");
        assert_eq!(haiku.context_length, 200_000);
        assert_eq!(haiku.input_cost_per_million, Some(0.25));
        assert_eq!(haiku.output_cost_per_million, Some(1.25));
    }

    #[test]
    fn test_all_models_support_features() {
        let models = get_models();
        for model in &models {
            assert!(model.supports_vision, "{} should support vision", model.id);
            assert!(model.supports_tools, "{} should support tools", model.id);
            assert!(model.supports_system, "{} should support system", model.id);
        }
    }

    #[test]
    fn test_all_models_have_costs() {
        let models = get_models();
        for model in &models {
            assert!(model.input_cost_per_million.is_some(), "{} should have input cost", model.id);
            assert!(model.output_cost_per_million.is_some(), "{} should have output cost", model.id);
        }
    }

    #[test]
    fn test_model_descriptions() {
        let models = get_models();
        for model in &models {
            assert!(model.description.is_some(), "{} should have description", model.id);
            assert!(!model.description.as_ref().unwrap().is_empty());
        }
    }
}
