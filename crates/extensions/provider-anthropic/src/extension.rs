//! Anthropic extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::AnthropicProvider;

/// Anthropic extension providing Claude models.
pub struct AnthropicExtension {
    manifest: ExtensionManifest,
}

impl AnthropicExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "provider-anthropic",
            "Anthropic Provider",
            Version::new(0, 1, 0),
        );
        manifest.description = "Anthropic Claude models".to_string();
        manifest.provides = Provides {
            providers: vec!["anthropic".to_string()],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for AnthropicExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for AnthropicExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let api_key: String = ctx
            .get_config("api_key")
            .ok_or_else(|| ExtensionError::InitializationFailed(
                "Missing api_key in config".to_string()
            ))?;

        let provider = AnthropicProvider::new(api_key);
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
    fn test_extension_new() {
        let ext = AnthropicExtension::new();
        assert_eq!(ext.manifest().id, "provider-anthropic");
        assert_eq!(ext.manifest().name, "Anthropic Provider");
    }

    #[test]
    fn test_extension_default() {
        let ext = AnthropicExtension::default();
        assert_eq!(ext.manifest().id, "provider-anthropic");
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = AnthropicExtension::new();
        assert_eq!(ext.manifest().version.major, 0);
        assert_eq!(ext.manifest().version.minor, 1);
        assert_eq!(ext.manifest().version.patch, 0);
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = AnthropicExtension::new();
        assert!(ext.manifest().description.contains("Claude"));
    }

    #[test]
    fn test_extension_manifest_provides() {
        let ext = AnthropicExtension::new();
        assert!(ext.manifest().provides.providers.contains(&"anthropic".to_string()));
    }

    #[test]
    fn test_extension_as_any() {
        let ext = AnthropicExtension::new();
        let any = ext.as_any();
        assert!(any.is::<AnthropicExtension>());
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = AnthropicExtension::new();
        let any_mut = ext.as_any_mut();
        assert!(any_mut.is::<AnthropicExtension>());
    }
}
