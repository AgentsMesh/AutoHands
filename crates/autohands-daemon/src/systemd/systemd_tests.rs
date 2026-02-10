use super::*;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = SystemdConfig::default();
    assert_eq!(config.service_name, "autohands");
    assert_eq!(config.restart, "on-failure");
    assert!(config.user_mode);
}

#[test]
fn test_config_builder() {
    let config = SystemdConfig::with_name("myservice")
        .exec_start("/usr/bin/myservice")
        .working_directory("/var/lib/myservice")
        .user("myuser")
        .env("FOO", "bar")
        .system_mode();

    assert_eq!(config.service_name, "myservice");
    assert_eq!(config.exec_start, PathBuf::from("/usr/bin/myservice"));
    assert_eq!(config.working_directory, Some(PathBuf::from("/var/lib/myservice")));
    assert_eq!(config.user, Some("myuser".to_string()));
    assert_eq!(config.environment.get("FOO"), Some(&"bar".to_string()));
    assert!(!config.user_mode);
}

#[test]
fn test_generate_unit() {
    let config = SystemdConfig::with_name("testservice")
        .exec_start("/usr/bin/test")
        .exec_args(vec!["--daemon".to_string()])
        .working_directory("/tmp")
        .env("TEST_VAR", "test_value");

    let service = SystemdService::new(config);
    let unit = service.generate_unit();

    assert!(unit.contains("[Unit]"));
    assert!(unit.contains("[Service]"));
    assert!(unit.contains("[Install]"));
    assert!(unit.contains("Description="));
    assert!(unit.contains("ExecStart=/usr/bin/test --daemon"));
    assert!(unit.contains("WorkingDirectory=/tmp"));
    assert!(unit.contains("Environment=\"TEST_VAR=test_value\""));
    assert!(unit.contains("Restart=on-failure"));
}

#[test]
fn test_unit_path_user_mode() {
    let config = SystemdConfig::with_name("testservice").user_mode();
    let service = SystemdService::new(config);
    let path = service.unit_path();

    assert!(path.to_string_lossy().contains("systemd/user"));
    assert!(path.to_string_lossy().contains("testservice.service"));
}

#[test]
fn test_unit_path_system_mode() {
    let config = SystemdConfig::with_name("testservice").system_mode();
    let service = SystemdService::new(config);
    let path = service.unit_path();

    assert_eq!(path, PathBuf::from("/etc/systemd/system/testservice.service"));
}
