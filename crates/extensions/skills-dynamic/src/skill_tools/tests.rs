use autohands_protocols::Extension;
use crate::extension::{DynamicSkillsConfig, DynamicSkillsExtension};
use std::path::PathBuf;

#[test]
fn test_extension_manifest() {
    let ext = DynamicSkillsExtension::new();
    assert_eq!(ext.manifest().id, "skills-dynamic");
}

#[test]
fn test_config_default() {
    let config = DynamicSkillsConfig::default();
    assert!(config.hot_reload);
    assert!(config.use_managed);
    assert!(config.use_workspace);
}

#[test]
fn test_with_extra_dir() {
    let ext = DynamicSkillsExtension::new()
        .with_extra_dir(PathBuf::from("/custom/skills"));
    assert_eq!(ext.config.extra_dirs.len(), 1);
}

#[test]
fn test_with_hot_reload() {
    let ext = DynamicSkillsExtension::new().with_hot_reload(false);
    assert!(!ext.config.hot_reload);
}
