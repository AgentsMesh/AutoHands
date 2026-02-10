//! Base registry trait and implementation.
//!
//! Provides a generic registry pattern to reduce code duplication across
//! ToolRegistry, ProviderRegistry, MemoryRegistry, and ExtensionRegistry.

use dashmap::DashMap;
use std::sync::Arc;

use autohands_protocols::error::ExtensionError;

/// Trait for items that can be stored in a registry.
///
/// Each registerable item must provide a unique ID.
pub trait Registerable: Send + Sync {
    /// Returns the unique identifier for this item.
    fn registry_id(&self) -> &str;
}

/// Generic registry for managing items by ID.
///
/// This provides the common functionality shared by all registries:
/// - Thread-safe storage using DashMap
/// - Register/unregister operations with duplicate checking
/// - Get by ID
/// - List all items
///
/// # Type Parameters
///
/// * `T` - The trait object type to store (e.g., `dyn Tool`, `dyn LLMProvider`)
pub struct BaseRegistry<T: ?Sized + Registerable> {
    items: DashMap<String, Arc<T>>,
}

impl<T: ?Sized + Registerable> BaseRegistry<T> {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            items: DashMap::new(),
        }
    }

    /// Register an item.
    ///
    /// Returns an error if an item with the same ID is already registered.
    pub fn register(&self, item: Arc<T>) -> Result<(), ExtensionError> {
        let id = item.registry_id().to_string();

        if self.items.contains_key(&id) {
            return Err(ExtensionError::AlreadyRegistered(id));
        }

        self.items.insert(id, item);
        Ok(())
    }

    /// Unregister an item by ID.
    ///
    /// Returns an error if no item with the given ID exists.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.items
            .remove(id)
            .ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        Ok(())
    }

    /// Get an item by ID.
    pub fn get(&self, id: &str) -> Option<Arc<T>> {
        self.items.get(id).map(|item| item.clone())
    }

    /// Check if an item with the given ID is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.items.contains_key(id)
    }

    /// List all registered item IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.items.iter().map(|item| item.registry_id().to_string()).collect()
    }

    /// Get the number of registered items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over all items.
    ///
    /// Returns an iterator of (id, Arc<T>) pairs.
    pub fn iter(&self) -> impl Iterator<Item = Arc<T>> + '_ {
        self.items.iter().map(|entry| entry.value().clone())
    }
}

impl<T: ?Sized + Registerable> Default for BaseRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "base_tests.rs"]
mod tests;
