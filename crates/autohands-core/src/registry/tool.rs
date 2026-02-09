//! Tool registry for managing available tools.

use std::sync::Arc;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::ToolRegistryAccess;
use autohands_protocols::tool::{Tool, ToolDefinition};

use super::base::{BaseRegistry, Registerable};

// Implement Registerable for Tool trait objects
impl Registerable for dyn Tool {
    fn registry_id(&self) -> &str {
        &self.definition().id
    }
}

/// Registry for managing tools.
///
/// Built on `BaseRegistry` for consistent behavior.
pub struct ToolRegistry {
    inner: BaseRegistry<dyn Tool>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            inner: BaseRegistry::new(),
        }
    }

    /// Register a tool.
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<(), ExtensionError> {
        self.inner.register(tool)
    }

    /// Unregister a tool.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.inner.unregister(id)
    }

    /// Get a tool by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.inner.get(id)
    }

    /// List all tool definitions.
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.inner.iter().map(|t| t.definition().clone()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistryAccess for ToolRegistry {
    fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<(), ExtensionError> {
        self.register(tool)
    }

    fn unregister_tool(&self, tool_id: &str) -> Result<(), ExtensionError> {
        self.unregister(tool_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::error::ToolError;
    use autohands_protocols::tool::{ToolContext, ToolResult};

    struct MockTool {
        definition: ToolDefinition,
    }

    impl MockTool {
        fn new(id: &str) -> Self {
            Self {
                definition: ToolDefinition::new(id, "Mock", "A mock tool"),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("executed"))
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_register_tool() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test-tool"));

        let result = registry.register(tool);
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = ToolRegistry::new();
        let tool1 = Arc::new(MockTool::new("test-tool"));
        let tool2 = Arc::new(MockTool::new("test-tool"));

        registry.register(tool1).unwrap();
        let result = registry.register(tool2);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_tool() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test-tool"));

        registry.register(tool).unwrap();
        let result = registry.unregister("test-tool");
        assert!(result.is_ok());
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = ToolRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_tool() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test-tool"));

        registry.register(tool).unwrap();
        let retrieved = registry.get("test-tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().definition().id, "test-tool");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_tools() {
        let registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool::new("tool1"))).unwrap();
        registry.register(Arc::new(MockTool::new("tool2"))).unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_tool_registry_access_trait() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test-tool"));

        // Test trait methods
        registry.register_tool(tool).unwrap();
        registry.unregister_tool("test-tool").unwrap();
    }
}
