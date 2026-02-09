//! Memory backend registry.

use std::sync::Arc;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::MemoryRegistryAccess;
use autohands_protocols::memory::MemoryBackend;

use super::base::{BaseRegistry, Registerable};

// Implement Registerable for MemoryBackend trait objects
impl Registerable for dyn MemoryBackend {
    fn registry_id(&self) -> &str {
        self.id()
    }
}

/// Registry for managing memory backends.
///
/// Built on `BaseRegistry` for consistent behavior.
pub struct MemoryRegistry {
    inner: BaseRegistry<dyn MemoryBackend>,
}

impl MemoryRegistry {
    /// Create a new memory registry.
    pub fn new() -> Self {
        Self {
            inner: BaseRegistry::new(),
        }
    }

    /// Register a memory backend.
    pub fn register(&self, backend: Arc<dyn MemoryBackend>) -> Result<(), ExtensionError> {
        self.inner.register(backend)
    }

    /// Unregister a memory backend.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.inner.unregister(id)
    }

    /// Get a memory backend by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn MemoryBackend>> {
        self.inner.get(id)
    }

    /// List all backend IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.inner.list_ids()
    }
}

impl Default for MemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRegistryAccess for MemoryRegistry {
    fn register_backend(&self, backend: Arc<dyn MemoryBackend>) -> Result<(), ExtensionError> {
        self.register(backend)
    }

    fn unregister_backend(&self, backend_id: &str) -> Result<(), ExtensionError> {
        self.unregister(backend_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::error::MemoryError;
    use autohands_protocols::memory::{MemoryEntry, MemoryQuery, MemorySearchResult};

    struct MockMemoryBackend {
        id: String,
    }

    impl MockMemoryBackend {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    #[async_trait]
    impl MemoryBackend for MockMemoryBackend {
        fn id(&self) -> &str {
            &self.id
        }

        async fn store(&self, _entry: MemoryEntry) -> Result<String, MemoryError> {
            Ok("test-id".to_string())
        }

        async fn retrieve(&self, _id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
            Ok(None)
        }

        async fn search(&self, _query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
            Ok(Vec::new())
        }

        async fn delete(&self, _id: &str) -> Result<(), MemoryError> {
            Ok(())
        }

        async fn update(&self, _id: &str, _entry: MemoryEntry) -> Result<(), MemoryError> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = MemoryRegistry::new();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = MemoryRegistry::default();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_register_backend() {
        let registry = MemoryRegistry::new();
        let backend = Arc::new(MockMemoryBackend::new("test-backend"));

        let result = registry.register(backend);
        assert!(result.is_ok());
        assert_eq!(registry.list_ids().len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = MemoryRegistry::new();
        let backend1 = Arc::new(MockMemoryBackend::new("test-backend"));
        let backend2 = Arc::new(MockMemoryBackend::new("test-backend"));

        registry.register(backend1).unwrap();
        let result = registry.register(backend2);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_backend() {
        let registry = MemoryRegistry::new();
        let backend = Arc::new(MockMemoryBackend::new("test-backend"));

        registry.register(backend).unwrap();
        let result = registry.unregister("test-backend");
        assert!(result.is_ok());
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = MemoryRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_backend() {
        let registry = MemoryRegistry::new();
        let backend = Arc::new(MockMemoryBackend::new("test-backend"));

        registry.register(backend).unwrap();
        let retrieved = registry.get("test-backend");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-backend");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = MemoryRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_ids() {
        let registry = MemoryRegistry::new();
        registry.register(Arc::new(MockMemoryBackend::new("backend1"))).unwrap();
        registry.register(Arc::new(MockMemoryBackend::new("backend2"))).unwrap();

        let ids = registry.list_ids();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_memory_registry_access_trait() {
        let registry = MemoryRegistry::new();
        let backend = Arc::new(MockMemoryBackend::new("test-backend"));

        registry.register_backend(backend).unwrap();
        registry.unregister_backend("test-backend").unwrap();
    }
}
