//! Skill tools extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::skill::SkillLoader;
use autohands_protocols::types::Version;

use crate::{SkillListTool, SkillLoadTool, SkillReadTool};

/// Skill tools extension providing skill discovery and loading capabilities.
///
/// This extension registers three tools:
/// - `skill_list`: List available skills
/// - `skill_load`: Load a skill's expert guidance
/// - `skill_read`: Read files from within a skill directory
pub struct SkillToolsExtension {
    manifest: ExtensionManifest,
    loader: Arc<RwLock<dyn SkillLoader>>,
}

impl SkillToolsExtension {
    pub fn new(loader: Arc<RwLock<dyn SkillLoader>>) -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-skill",
            "Skill Tools",
            Version::new(0, 1, 0),
        );
        manifest.description = "Dynamic skill discovery, loading, and resource access".to_string();
        manifest.provides = Provides {
            tools: vec![
                "skill_list".to_string(),
                "skill_load".to_string(),
                "skill_read".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest, loader }
    }
}

#[async_trait]
impl Extension for SkillToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        ctx.tool_registry
            .register_tool(Arc::new(SkillListTool::new(self.loader.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(SkillLoadTool::new(self.loader.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(SkillReadTool::new(self.loader.clone())))?;

        tracing::info!("Skill tools registered: skill_list, skill_load, skill_read");
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
    use autohands_protocols::skill::{Skill, SkillDefinition};
    use autohands_protocols::error::SkillError;

    struct MockLoader;

    #[async_trait]
    impl SkillLoader for MockLoader {
        async fn load(&self, _skill_id: &str) -> Result<Skill, SkillError> {
            let def = SkillDefinition::new("test", "Test");
            Ok(Skill::new(def, "content"))
        }

        async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError> {
            Ok(vec![])
        }

        async fn reload(&self) -> Result<(), SkillError> {
            Ok(())
        }
    }

    #[test]
    fn test_extension_manifest() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader));
        let ext = SkillToolsExtension::new(loader);

        assert_eq!(ext.manifest().id, "tools-skill");
        assert_eq!(ext.manifest().name, "Skill Tools");
        assert!(ext.manifest().provides.tools.contains(&"skill_list".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"skill_load".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"skill_read".to_string()));
    }

    #[test]
    fn test_extension_provides_count() {
        let loader: Arc<RwLock<dyn SkillLoader>> = Arc::new(RwLock::new(MockLoader));
        let ext = SkillToolsExtension::new(loader);
        assert_eq!(ext.manifest().provides.tools.len(), 3);
    }
}
