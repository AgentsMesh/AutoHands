
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert!(config.enabled);
        assert!(config.auto_restart);
        assert_eq!(config.max_restarts, 10);
        assert!(config.daemonize);
    }

    #[test]
    fn test_config_with_pid_file() {
        let config = DaemonConfig::with_pid_file(PathBuf::from("/tmp/test.pid"));
        assert_eq!(config.pid_file, PathBuf::from("/tmp/test.pid"));
    }

    #[test]
    fn test_duration_getters() {
        let config = DaemonConfig::default();
        assert_eq!(config.restart_window(), Duration::from_secs(300));
        assert_eq!(config.restart_delay(), Duration::from_secs(5));
        assert_eq!(config.health_check_interval(), Duration::from_secs(30));
        assert_eq!(config.shutdown_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = DaemonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_max_restarts() {
        let mut config = DaemonConfig::default();
        config.max_restarts = 0;
        config.auto_restart = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_restart_window() {
        let mut config = DaemonConfig::default();
        config.restart_window_secs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = DaemonConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains("auto_restart"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"enabled": false, "max_restarts": 5}"#;
        let config: DaemonConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.max_restarts, 5);
    }
