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

    async fn shutdown(&self) -> Result<(), ExtensionError> {
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
#[path = "extension_tests.rs"]
mod tests;
