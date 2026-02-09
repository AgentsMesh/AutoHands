//! OpenAI extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{
    Extension, ExtensionContext, ExtensionManifest, Provides,
};
use autohands_protocols::types::Version;

use crate::OpenAIProvider;

/// OpenAI extension providing GPT models.
pub struct OpenAIExtension {
    manifest: ExtensionManifest,
    api_key: Option<String>,
    api_url: Option<String>,
}

impl OpenAIExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "provider-openai",
            "OpenAI Provider",
            Version::new(0, 1, 0),
        );
        manifest.description = "OpenAI GPT models provider".to_string();
        manifest.provides = Provides {
            providers: vec!["openai".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            api_key: None,
            api_url: None,
        }
    }

    /// Set API key for authentication.
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set custom API URL (for OpenAI-compatible APIs).
    pub fn with_api_url(mut self, api_url: String) -> Self {
        self.api_url = Some(api_url);
        self
    }
}

impl Default for OpenAIExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for OpenAIExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let api_key = self
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| ExtensionError::InitializationFailed("OPENAI_API_KEY not set".to_string()))?;

        let provider = if let Some(url) = &self.api_url {
            OpenAIProvider::with_url(api_key, url.clone())
        } else {
            OpenAIProvider::new(api_key)
        };

        ctx.provider_registry.register_provider(Arc::new(provider))?;
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
        let ext = OpenAIExtension::new();
        assert_eq!(ext.manifest().id, "provider-openai");
        assert!(ext.manifest().provides.providers.contains(&"openai".to_string()));
    }

    #[test]
    fn test_with_api_key() {
        let ext = OpenAIExtension::new().with_api_key("test-key".to_string());
        assert_eq!(ext.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_with_custom_url() {
        let ext = OpenAIExtension::new()
            .with_api_url("https://custom.api/v1".to_string());
        assert_eq!(ext.api_url, Some("https://custom.api/v1".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = OpenAIExtension::default();
        assert_eq!(ext.manifest().id, "provider-openai");
        assert!(ext.api_key.is_none());
        assert!(ext.api_url.is_none());
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = OpenAIExtension::new();
        assert!(!ext.manifest().description.is_empty());
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = OpenAIExtension::new();
        let version = &ext.manifest().version;
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_extension_builder_chain() {
        let ext = OpenAIExtension::new()
            .with_api_key("key123".to_string())
            .with_api_url("https://api.example.com".to_string());
        assert_eq!(ext.api_key, Some("key123".to_string()));
        assert_eq!(ext.api_url, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_extension_as_any() {
        let ext = OpenAIExtension::new();
        let _any: &dyn Any = ext.as_any();
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = OpenAIExtension::new();
        let _any: &mut dyn Any = ext.as_any_mut();
    }

    #[test]
    fn test_manifest_provides() {
        let ext = OpenAIExtension::new();
        let provides = &ext.manifest().provides;
        assert!(provides.tools.is_empty());
        assert!(!provides.providers.is_empty());
        assert!(provides.memory_backends.is_empty());
    }
}
