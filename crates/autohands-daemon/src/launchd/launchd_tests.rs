use super::*;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = LaunchAgentConfig::default();
    assert_eq!(config.label, "com.autohands.agent");
    assert!(config.run_at_load);
    assert!(config.keep_alive);
}

#[test]
fn test_config_builder() {
    let config = LaunchAgentConfig::with_label("com.test.service")
        .program("/usr/bin/test")
        .working_directory("/tmp")
        .env("FOO", "bar");

    assert_eq!(config.label, "com.test.service");
    assert_eq!(config.program, PathBuf::from("/usr/bin/test"));
    assert_eq!(config.working_directory, Some(PathBuf::from("/tmp")));
    assert_eq!(config.environment_variables.get("FOO"), Some(&"bar".to_string()));
}

#[test]
fn test_generate_plist() {
    let config = LaunchAgentConfig::with_label("com.test.agent")
        .program("/usr/local/bin/test")
        .program_arguments(vec!["--daemon".to_string()])
        .env("PATH", "/usr/local/bin");

    let agent = LaunchAgent::new(config);
    let plist = agent.generate_plist();

    assert!(plist.contains("<key>Label</key>"));
    assert!(plist.contains("<string>com.test.agent</string>"));
    assert!(plist.contains("<key>Program</key>"));
    assert!(plist.contains("<string>/usr/local/bin/test</string>"));
    assert!(plist.contains("<key>EnvironmentVariables</key>"));
    assert!(plist.contains("<key>PATH</key>"));
}

#[test]
fn test_plist_path() {
    let config = LaunchAgentConfig::with_label("com.test.agent");
    let agent = LaunchAgent::new(config);
    let path = agent.plist_path();

    assert!(path.to_string_lossy().contains("LaunchAgents"));
    assert!(path.to_string_lossy().contains("com.test.agent.plist"));
}

#[test]
fn test_escape_xml() {
    use super::launchd_agent::escape_xml;
    assert_eq!(escape_xml("foo & bar"), "foo &amp; bar");
    assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
}
