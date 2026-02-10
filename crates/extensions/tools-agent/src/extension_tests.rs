use super::*;

#[test]
fn test_extension_manifest() {
    let ext = AgentToolsExtension::new();
    assert_eq!(ext.manifest().id, "tools-agent");
    assert!(ext
        .manifest()
        .provides
        .tools
        .contains(&"agent_spawn".to_string()));
}

#[test]
fn test_config_default() {
    let config = AgentToolsConfig::default();
    assert_eq!(config.max_concurrent, 10);
}

#[test]
fn test_max_concurrent_builder() {
    let ext = AgentToolsExtension::new().max_concurrent(5);
    assert_eq!(ext.config.max_concurrent, 5);
}

#[test]
fn test_extension_default() {
    let ext = AgentToolsExtension::default();
    assert_eq!(ext.manifest().id, "tools-agent");
}

#[test]
fn test_manager_initially_none() {
    let ext = AgentToolsExtension::new();
    assert!(ext.manager().is_none());
}

#[test]
fn test_as_any() {
    let ext = AgentToolsExtension::new();
    let any_ref = ext.as_any();
    assert!(any_ref.downcast_ref::<AgentToolsExtension>().is_some());
}

#[test]
fn test_manifest_tools_count() {
    let ext = AgentToolsExtension::new();
    assert_eq!(ext.manifest().provides.tools.len(), 5);
}

#[test]
fn test_manifest_description() {
    let ext = AgentToolsExtension::new();
    assert!(ext
        .manifest()
        .description
        .contains("Sub-agent"));
}
