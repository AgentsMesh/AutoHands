//! In-memory workflow store.

use std::collections::HashMap;

use tokio::sync::RwLock;

use super::definition::Workflow;
use crate::error::InterfaceError;

/// Trait for workflow persistence.
#[async_trait::async_trait]
pub trait WorkflowStore: Send + Sync {
    /// Save a workflow.
    async fn save(&self, workflow: &Workflow) -> Result<(), InterfaceError>;

    /// Load a workflow by ID.
    async fn load(&self, id: &str) -> Result<Option<Workflow>, InterfaceError>;

    /// Load all workflows.
    async fn load_all(&self) -> Result<Vec<Workflow>, InterfaceError>;

    /// Delete a workflow by ID.
    async fn delete(&self, id: &str) -> Result<bool, InterfaceError>;
}

/// In-memory workflow store.
pub struct MemoryWorkflowStore {
    workflows: RwLock<HashMap<String, Workflow>>,
}

impl MemoryWorkflowStore {
    /// Create a new memory workflow store.
    pub fn new() -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryWorkflowStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl WorkflowStore for MemoryWorkflowStore {
    async fn save(&self, workflow: &Workflow) -> Result<(), InterfaceError> {
        let mut store = self.workflows.write().await;
        store.insert(workflow.id.clone(), workflow.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Workflow>, InterfaceError> {
        let store = self.workflows.read().await;
        Ok(store.get(id).cloned())
    }

    async fn load_all(&self) -> Result<Vec<Workflow>, InterfaceError> {
        let store = self.workflows.read().await;
        Ok(store.values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> Result<bool, InterfaceError> {
        let mut store = self.workflows.write().await;
        Ok(store.remove(id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::definition::{Workflow, WorkflowStep};

    #[tokio::test]
    async fn test_memory_workflow_store_crud() {
        let store = MemoryWorkflowStore::new();
        let step = WorkflowStep::agent("s1", "Step 1", "test-agent", "Do something");
        let workflow = Workflow::new("wf-1", "Test Workflow", step);

        // Save
        store.save(&workflow).await.unwrap();

        // Load
        let loaded = store.load("wf-1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "Test Workflow");

        // Load all
        let all = store.load_all().await.unwrap();
        assert_eq!(all.len(), 1);

        // Load non-existent
        let missing = store.load("non-existent").await.unwrap();
        assert!(missing.is_none());

        // Delete
        let deleted = store.delete("wf-1").await.unwrap();
        assert!(deleted);

        let deleted_again = store.delete("wf-1").await.unwrap();
        assert!(!deleted_again);

        let all = store.load_all().await.unwrap();
        assert!(all.is_empty());
    }
}
