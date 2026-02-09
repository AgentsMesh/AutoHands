//! General agent extension definition.

use std::any::Any;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

/// General agent extension.
pub struct GeneralAgentExtension {
    manifest: ExtensionManifest,
}

impl GeneralAgentExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "agent-general",
            "General Agent",
            Version::new(0, 1, 0),
        );
        manifest.description = "General purpose agentic execution".to_string();
        manifest.provides = Provides {
            agents: vec!["general".to_string()],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for GeneralAgentExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for GeneralAgentExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Agent registration would go here when we have an AgentRegistry
        // For now, agents are created on-demand by the runtime
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
        let ext = GeneralAgentExtension::new();
        assert_eq!(ext.manifest().id, "agent-general");
        assert!(ext.manifest().provides.agents.contains(&"general".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = GeneralAgentExtension::default();
        assert_eq!(ext.manifest().id, "agent-general");
    }

    #[test]
    fn test_manifest_name() {
        let ext = GeneralAgentExtension::new();
        assert_eq!(ext.manifest().name, "General Agent");
    }

    #[test]
    fn test_manifest_description() {
        let ext = GeneralAgentExtension::new();
        assert_eq!(ext.manifest().description, "General purpose agentic execution");
    }

    #[test]
    fn test_manifest_version() {
        let ext = GeneralAgentExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = GeneralAgentExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<GeneralAgentExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = GeneralAgentExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<GeneralAgentExtension>().is_some());
    }

    #[test]
    fn test_provides_agents() {
        let ext = GeneralAgentExtension::new();
        assert_eq!(ext.manifest().provides.agents.len(), 1);
        assert_eq!(ext.manifest().provides.agents[0], "general");
    }

    #[test]
    fn test_provides_no_tools() {
        let ext = GeneralAgentExtension::new();
        assert!(ext.manifest().provides.tools.is_empty());
    }
}
