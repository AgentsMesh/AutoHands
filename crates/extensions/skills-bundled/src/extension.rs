//! Bundled skills extension definition.

use std::any::Any;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

/// Bundled skills extension.
pub struct BundledSkillsExtension {
    manifest: ExtensionManifest,
}

impl BundledSkillsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "skills-bundled",
            "Bundled Skills",
            Version::new(0, 1, 0),
        );
        manifest.description = "Built-in skills for common development tasks".to_string();
        manifest.provides = Provides {
            skills: vec![
                "code-review".to_string(),
                "explain-code".to_string(),
                "write-tests".to_string(),
                "refactor".to_string(),
                "debug".to_string(),
                "documentation".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for BundledSkillsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for BundledSkillsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Skill registration would go here when we have a SkillRegistry
        // For now, skills are loaded on-demand by the BundledSkillLoader
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
        let ext = BundledSkillsExtension::new();
        assert_eq!(ext.manifest().id, "skills-bundled");
        assert!(ext.manifest().provides.skills.contains(&"code-review".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = BundledSkillsExtension::default();
        assert_eq!(ext.manifest().id, "skills-bundled");
    }

    #[test]
    fn test_manifest_name() {
        let ext = BundledSkillsExtension::new();
        assert_eq!(ext.manifest().name, "Bundled Skills");
    }

    #[test]
    fn test_manifest_description() {
        let ext = BundledSkillsExtension::new();
        assert_eq!(ext.manifest().description, "Built-in skills for common development tasks");
    }

    #[test]
    fn test_manifest_version() {
        let ext = BundledSkillsExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = BundledSkillsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<BundledSkillsExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = BundledSkillsExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<BundledSkillsExtension>().is_some());
    }

    #[test]
    fn test_all_bundled_skills_present() {
        let ext = BundledSkillsExtension::new();
        let skills = &ext.manifest().provides.skills;
        assert!(skills.contains(&"code-review".to_string()));
        assert!(skills.contains(&"explain-code".to_string()));
        assert!(skills.contains(&"write-tests".to_string()));
        assert!(skills.contains(&"refactor".to_string()));
        assert!(skills.contains(&"debug".to_string()));
        assert!(skills.contains(&"documentation".to_string()));
    }

    #[test]
    fn test_provides_no_tools() {
        let ext = BundledSkillsExtension::new();
        assert!(ext.manifest().provides.tools.is_empty());
    }

    #[test]
    fn test_skills_count() {
        let ext = BundledSkillsExtension::new();
        assert_eq!(ext.manifest().provides.skills.len(), 6);
    }
}
