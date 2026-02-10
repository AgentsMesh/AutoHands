use super::*;

#[test]
fn test_extension_manifest() {
    let ext = BrowserToolsExtension::new();
    assert_eq!(ext.manifest().id, "tools-browser");
    assert!(ext
        .manifest()
        .provides
        .tools
        .contains(&"browser_click".to_string()));
}

#[test]
fn test_builder_methods() {
    let ext = BrowserToolsExtension::new().viewport(1920, 1080);

    assert_eq!(ext.config.viewport_width, 1920);
    assert_eq!(ext.config.viewport_height, 1080);
}

#[test]
fn test_extension_default() {
    let ext = BrowserToolsExtension::default();
    assert_eq!(ext.manifest().id, "tools-browser");
}

#[test]
fn test_manifest_name() {
    let ext = BrowserToolsExtension::new();
    assert_eq!(ext.manifest().name, "Browser Tools");
}

#[test]
fn test_manifest_description() {
    let ext = BrowserToolsExtension::new();
    assert!(ext.manifest().description.contains("CDP"));
}

#[test]
fn test_manifest_version() {
    let ext = BrowserToolsExtension::new();
    assert_eq!(ext.manifest().version, Version::new(0, 4, 0));
}

#[test]
fn test_all_tools_provided() {
    let ext = BrowserToolsExtension::new();
    let tools = &ext.manifest().provides.tools;
    assert!(tools.contains(&"browser_navigate".to_string()));
    assert!(tools.contains(&"browser_click".to_string()));
    assert!(tools.contains(&"browser_type".to_string()));
    assert!(tools.contains(&"browser_screenshot".to_string()));
    assert!(tools.contains(&"browser_get_content".to_string()));
    assert!(tools.contains(&"browser_execute_js".to_string()));
    assert!(tools.contains(&"browser_wait_for".to_string()));
    assert!(tools.contains(&"browser_get_dom".to_string()));
}

#[test]
fn test_tools_count() {
    let ext = BrowserToolsExtension::new();
    // 16 basic + 1 DOM + 3 AI = 20 tools
    assert_eq!(ext.manifest().provides.tools.len(), 20);
}

#[test]
fn test_manager_initially_none() {
    let ext = BrowserToolsExtension::new();
    assert!(ext.manager().is_none());
}

#[test]
fn test_as_any() {
    let ext = BrowserToolsExtension::new();
    let any_ref = ext.as_any();
    assert!(any_ref.downcast_ref::<BrowserToolsExtension>().is_some());
}

#[test]
fn test_as_any_mut() {
    let mut ext = BrowserToolsExtension::new();
    let any_ref = ext.as_any_mut();
    assert!(any_ref.downcast_mut::<BrowserToolsExtension>().is_some());
}

#[test]
fn test_builder_chain() {
    let ext = BrowserToolsExtension::new()
        .viewport(800, 600)
        .debug_port(9333)
        .headless(true);

    assert_eq!(ext.config.viewport_width, 800);
    assert_eq!(ext.config.viewport_height, 600);
    assert_eq!(ext.config.debug_port, 9333);
    assert!(ext.config.headless);
}

#[test]
fn test_default_port() {
    let ext = BrowserToolsExtension::new();
    assert_eq!(ext.config.debug_port, 9222);
}

#[test]
fn test_profile_dir() {
    let ext = BrowserToolsExtension::new()
        .profile_dir("/custom/profile");
    assert_eq!(ext.config.profile_dir, Some(PathBuf::from("/custom/profile")));
}
