//! Ark extension for AutoHands.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::provider::ModelDefinition;
use autohands_protocols::types::Version;

use crate::provider::ArkProvider;

/// Ark extension for AutoHands.
///
/// This extension registers the Ark provider (火山引擎方舟平台) which supports
/// Doubao (豆包) models.
///
/// # Configuration
///
/// The extension can be configured with:
/// - `api_key`: The Ark API key (or set `ARK_API_KEY` environment variable)
/// - `api_url`: Custom API URL (optional)
/// - `custom_models`: Additional model definitions (optional)
///
/// # Example
///
/// ```toml
/// [providers.ark]
/// api_key = "your-api-key"
/// ```
pub struct ArkExtension {
    manifest: ExtensionManifest,
    api_key: Option<String>,
    api_url: Option<String>,
    custom_models: Vec<ModelDefinition>,
}

impl ArkExtension {
    /// Create a new Ark extension.
    pub fn new() -> Self {
        let mut manifest =
            ExtensionManifest::new("provider-ark", "Ark Provider", Version::new(0, 1, 0));
        manifest.description =
            "Ark (火山引擎方舟) LLM provider supporting Doubao models".to_string();
        manifest.provides = Provides {
            providers: vec!["ark".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            api_key: None,
            api_url: None,
            custom_models: Vec::new(),
        }
    }

    /// Create with an API key.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set a custom API URL.
    pub fn with_api_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = Some(url.into());
        self
    }

    /// Add a custom model definition.
    ///
    /// This is useful when you need to use a specific endpoint ID
    /// (e.g., `ep-xxxxxxxx-xxxxx`) instead of the standard model names.
    pub fn with_custom_model(mut self, model: ModelDefinition) -> Self {
        self.custom_models.push(model);
        self
    }
}

impl Default for ArkExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for ArkExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Try to get API key from:
        // 1. Constructor parameter
        // 2. Config
        // 3. Environment variable
        let api_key = self
            .api_key
            .clone()
            .or_else(|| {
                ctx.config
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| std::env::var("ARK_API_KEY").ok())
            .ok_or_else(|| {
                ExtensionError::InitializationFailed(
                    "ARK_API_KEY not set. Please provide api_key in config or set ARK_API_KEY environment variable.".to_string(),
                )
            })?;

        // Check for custom API URL
        let api_url = self.api_url.clone().or_else(|| {
            ctx.config
                .get("api_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        // Create the provider
        let mut provider = if let Some(url) = api_url {
            info!("Using custom Ark API URL: {}", url);
            ArkProvider::with_url(api_key, url)
        } else {
            ArkProvider::new(api_key)
        };

        // Add custom models from config
        if let Some(models) = ctx.config.get("custom_models").and_then(|v| v.as_array()) {
            for model_value in models {
                if let Ok(model) = serde_json::from_value::<ModelDefinition>(model_value.clone()) {
                    info!("Adding custom model: {}", model.id);
                    provider = provider.with_custom_model(model);
                }
            }
        }

        // Add custom models from constructor
        for model in self.custom_models.drain(..) {
            info!("Adding custom model: {}", model.id);
            provider = provider.with_custom_model(model);
        }

        // Register the provider
        ctx.provider_registry.register_provider(Arc::new(provider))?;

        info!("Ark provider initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ExtensionError> {
        info!("Ark provider shutting down");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_manifest() {
        let ext = ArkExtension::new();
        assert_eq!(ext.manifest().id, "provider-ark");
        assert!(ext.manifest().provides.providers.contains(&"ark".to_string()));
    }

    #[test]
    fn test_extension_with_api_key() {
        let ext = ArkExtension::new().with_api_key("test-key");
        assert_eq!(ext.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_extension_with_api_url() {
        let ext = ArkExtension::new().with_api_url("https://custom.api");
        assert_eq!(ext.api_url, Some("https://custom.api".to_string()));
    }

    #[test]
    fn test_extension_with_custom_model() {
        let model = ModelDefinition {
            id: "ep-test-endpoint".to_string(),
            name: "Test Endpoint".to_string(),
            description: None,
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: None,
            output_cost_per_million: None,
            metadata: Default::default(),
        };
        let ext = ArkExtension::new().with_custom_model(model);
        assert_eq!(ext.custom_models.len(), 1);
        assert_eq!(ext.custom_models[0].id, "ep-test-endpoint");
    }

    #[test]
    fn test_default_extension() {
        let ext = ArkExtension::default();
        assert!(ext.api_key.is_none());
        assert!(ext.api_url.is_none());
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = ArkExtension::new();
        assert!(!ext.manifest().description.is_empty());
        assert!(ext.manifest().description.contains("Ark"));
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = ArkExtension::new();
        let version = &ext.manifest().version;
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_extension_builder_chain() {
        let ext = ArkExtension::new()
            .with_api_key("key123")
            .with_api_url("https://api.example.com");
        assert_eq!(ext.api_key, Some("key123".to_string()));
        assert_eq!(ext.api_url, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_extension_as_any() {
        let ext = ArkExtension::new();
        let _any: &dyn Any = ext.as_any();
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = ArkExtension::new();
        let _any: &mut dyn Any = ext.as_any_mut();
    }

    #[test]
    fn test_manifest_provides() {
        let ext = ArkExtension::new();
        let provides = &ext.manifest().provides;
        assert!(provides.tools.is_empty());
        assert!(!provides.providers.is_empty());
        assert!(provides.memory_backends.is_empty());
    }
}
