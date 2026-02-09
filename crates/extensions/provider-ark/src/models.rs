//! Ark model definitions.
//!
//! This module defines the available models on the Ark platform (火山引擎方舟).
//! Ark supports various Doubao (豆包) models with different capabilities.

use autohands_protocols::provider::ModelDefinition;

/// Get the list of available Ark models.
pub fn get_models() -> Vec<ModelDefinition> {
    vec![
        // Doubao Pro 系列 - 高性能模型
        ModelDefinition {
            id: "doubao-pro-32k".to_string(),
            name: "Doubao Pro 32K".to_string(),
            description: Some("豆包 Pro 32K 高性能通用模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(0.8),
            output_cost_per_million: Some(2.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "doubao-pro-128k".to_string(),
            name: "Doubao Pro 128K".to_string(),
            description: Some("豆包 Pro 128K 长上下文高性能模型".to_string()),
            context_length: 131072,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(5.0),
            output_cost_per_million: Some(9.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "doubao-pro-256k".to_string(),
            name: "Doubao Pro 256K".to_string(),
            description: Some("豆包 Pro 256K 超长上下文模型".to_string()),
            context_length: 262144,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(5.0),
            output_cost_per_million: Some(9.0),
            metadata: Default::default(),
        },
        // Doubao Lite 系列 - 轻量级模型
        ModelDefinition {
            id: "doubao-lite-32k".to_string(),
            name: "Doubao Lite 32K".to_string(),
            description: Some("豆包 Lite 32K 轻量级模型，性价比高".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(0.3),
            output_cost_per_million: Some(0.6),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "doubao-lite-128k".to_string(),
            name: "Doubao Lite 128K".to_string(),
            description: Some("豆包 Lite 128K 长上下文轻量级模型".to_string()),
            context_length: 131072,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(0.8),
            output_cost_per_million: Some(1.0),
            metadata: Default::default(),
        },
        // Doubao Vision 系列 - 视觉模型
        ModelDefinition {
            id: "doubao-vision-pro-32k".to_string(),
            name: "Doubao Vision Pro 32K".to_string(),
            description: Some("豆包视觉 Pro 32K 多模态模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(3.0),
            output_cost_per_million: Some(9.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "doubao-vision-lite-32k".to_string(),
            name: "Doubao Vision Lite 32K".to_string(),
            description: Some("豆包视觉 Lite 32K 轻量级多模态模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: true,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(1.5),
            output_cost_per_million: Some(4.5),
            metadata: Default::default(),
        },
        // Doubao Seed 系列 - 新一代模型
        ModelDefinition {
            id: "doubao-seed-1-6".to_string(),
            name: "Doubao Seed 1.6".to_string(),
            description: Some("豆包 Seed 1.6 新一代模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(1.0),
            output_cost_per_million: Some(2.0),
            metadata: Default::default(),
        },
        ModelDefinition {
            id: "doubao-seed-1-8-251228".to_string(),
            name: "Doubao Seed 1.8 (251228)".to_string(),
            description: Some("豆包 Seed 1.8 最新版本模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: Some(1.0),
            output_cost_per_million: Some(2.0),
            metadata: Default::default(),
        },
        // Character 系列 - 角色扮演模型
        ModelDefinition {
            id: "doubao-character-pro-32k".to_string(),
            name: "Doubao Character Pro 32K".to_string(),
            description: Some("豆包角色 Pro 32K 角色扮演专用模型".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: false,
            supports_system: true,
            input_cost_per_million: Some(2.0),
            output_cost_per_million: Some(4.0),
            metadata: Default::default(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_models_not_empty() {
        let models = get_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_models_have_doubao_pro() {
        let models = get_models();
        let has_pro = models.iter().any(|m| m.id.contains("doubao-pro"));
        assert!(has_pro);
    }

    #[test]
    fn test_models_have_seed() {
        let models = get_models();
        let has_seed = models.iter().any(|m| m.id.contains("doubao-seed"));
        assert!(has_seed);
    }

    #[test]
    fn test_vision_model_supports_vision() {
        let models = get_models();
        let vision_model = models.iter().find(|m| m.id.contains("vision"));
        assert!(vision_model.is_some());
        assert!(vision_model.unwrap().supports_vision);
    }

    #[test]
    fn test_model_context_lengths() {
        let models = get_models();
        for model in &models {
            assert!(model.context_length >= 32768);
        }
    }
}
