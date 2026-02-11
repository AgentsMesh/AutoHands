//! Webhook registry for managing webhook registrations.
//!
//! Uses `DashMap` for lock-free concurrent read/write access.

use dashmap::DashMap;

use super::types::WebhookRegistration;

/// Thread-safe registry for webhook registrations.
///
/// Backed by `DashMap` to support concurrent access from multiple
/// HTTP handler threads without explicit locking.
pub struct WebhookRegistry {
    registrations: DashMap<String, WebhookRegistration>,
}

impl WebhookRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            registrations: DashMap::new(),
        }
    }

    /// Register a webhook. Overwrites any existing registration with the same ID.
    pub fn register(&self, registration: WebhookRegistration) {
        self.registrations
            .insert(registration.id.clone(), registration);
    }

    /// Get a webhook registration by ID.
    pub fn get(&self, id: &str) -> Option<WebhookRegistration> {
        self.registrations.get(id).map(|r| r.value().clone())
    }

    /// Remove a webhook registration by ID.
    ///
    /// Returns the removed registration if it existed.
    pub fn remove(&self, id: &str) -> Option<WebhookRegistration> {
        self.registrations.remove(id).map(|(_, v)| v)
    }

    /// List all registered webhooks.
    pub fn list(&self) -> Vec<WebhookRegistration> {
        self.registrations
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Check if a webhook with the given ID exists.
    pub fn contains(&self, id: &str) -> bool {
        self.registrations.contains_key(id)
    }

    /// Get the number of registered webhooks.
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }
}

impl Default for WebhookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
