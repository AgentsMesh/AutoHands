//! Extension registry for managing loaded extensions.

use std::sync::Arc;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionManifest};

use super::base::{BaseRegistry, Registerable};

// Implement Registerable for Extension trait objects
impl Registerable for dyn Extension {
    fn registry_id(&self) -> &str {
        &self.manifest().id
    }
}

/// Registry for managing extensions.
///
/// Built on `BaseRegistry` for consistent behavior.
pub struct ExtensionRegistry {
    inner: BaseRegistry<dyn Extension>,
}

impl ExtensionRegistry {
    /// Create a new extension registry.
    pub fn new() -> Self {
        Self {
            inner: BaseRegistry::new(),
        }
    }

    /// Register an extension.
    pub fn register(&self, extension: Arc<dyn Extension>) -> Result<(), ExtensionError> {
        self.inner.register(extension)
    }

    /// Unregister an extension.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.inner.unregister(id)
    }

    /// Get an extension by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Extension>> {
        self.inner.get(id)
    }

    /// List all registered extensions.
    pub fn list(&self) -> Vec<ExtensionManifest> {
        self.inner.iter().map(|e| e.manifest().clone()).collect()
    }

    /// Check if an extension is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::extension::ExtensionContext;
    use autohands_protocols::types::Version;

    struct MockExtension {
        manifest: ExtensionManifest,
    }

    impl MockExtension {
        fn new(id: &str) -> Self {
            Self {
                manifest: ExtensionManifest::new(id, format!("Mock {}", id), Version::new(1, 0, 0))
                    .with_description("A mock extension"),
            }
        }
    }

    #[async_trait]
    impl Extension for MockExtension {
        fn manifest(&self) -> &ExtensionManifest {
            &self.manifest
        }

        async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), ExtensionError> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ExtensionRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = ExtensionRegistry::default();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_register_extension() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(MockExtension::new("test-ext"));

        let result = registry.register(ext);
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = ExtensionRegistry::new();
        let ext1 = Arc::new(MockExtension::new("test-ext"));
        let ext2 = Arc::new(MockExtension::new("test-ext"));

        registry.register(ext1).unwrap();
        let result = registry.register(ext2);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_extension() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(MockExtension::new("test-ext"));

        registry.register(ext).unwrap();
        let result = registry.unregister("test-ext");
        assert!(result.is_ok());
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = ExtensionRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_extension() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(MockExtension::new("test-ext"));

        registry.register(ext).unwrap();
        let retrieved = registry.get("test-ext");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().manifest().id, "test-ext");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = ExtensionRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_contains() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(MockExtension::new("test-ext"));

        assert!(!registry.contains("test-ext"));
        registry.register(ext).unwrap();
        assert!(registry.contains("test-ext"));
    }

    #[test]
    fn test_list_extensions() {
        let registry = ExtensionRegistry::new();
        registry.register(Arc::new(MockExtension::new("ext1"))).unwrap();
        registry.register(Arc::new(MockExtension::new("ext2"))).unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }
}
