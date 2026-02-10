use super::*;

#[test]
fn test_risk_level_default() {
    let level = RiskLevel::default();
    assert_eq!(level, RiskLevel::Low);
}

#[test]
fn test_risk_level_ordering() {
    assert!(RiskLevel::Low < RiskLevel::Medium);
    assert!(RiskLevel::Medium < RiskLevel::High);
    assert!(RiskLevel::Low < RiskLevel::High);
}

#[test]
fn test_risk_level_serialization() {
    let level = RiskLevel::High;
    let json = serde_json::to_string(&level).unwrap();
    assert_eq!(json, "\"high\"");

    let parsed: RiskLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, RiskLevel::High);
}

#[test]
fn test_stop_reason_serialization() {
    let reason = StopReason::EndTurn;
    let json = serde_json::to_string(&reason).unwrap();
    assert_eq!(json, "\"end_turn\"");

    let reason = StopReason::MaxTokens;
    let json = serde_json::to_string(&reason).unwrap();
    assert_eq!(json, "\"max_tokens\"");
}

#[test]
fn test_usage_default() {
    let usage = Usage::default();
    assert_eq!(usage.prompt_tokens, 0);
    assert_eq!(usage.completion_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
    assert!(usage.cache_creation_tokens.is_none());
    assert!(usage.cache_read_tokens.is_none());
}

#[test]
fn test_usage_serialization() {
    let usage = Usage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
        cache_creation_tokens: Some(50),
        cache_read_tokens: Some(25),
    };
    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("100"));
    assert!(json.contains("200"));
    assert!(json.contains("300"));
}

#[test]
fn test_author() {
    let author = Author {
        name: "Test Author".to_string(),
        email: Some("test@example.com".to_string()),
        url: Some("https://example.com".to_string()),
    };
    let json = serde_json::to_string(&author).unwrap();
    assert!(json.contains("Test Author"));
    assert!(json.contains("test@example.com"));
}

#[test]
fn test_version_new() {
    let version = Version::new(1, 2, 3);
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 3);
    assert!(version.prerelease.is_none());
}

#[test]
fn test_version_display() {
    let version = Version::new(1, 2, 3);
    assert_eq!(format!("{}", version), "1.2.3");

    let version = Version {
        major: 1,
        minor: 0,
        patch: 0,
        prerelease: Some("beta".to_string()),
    };
    assert_eq!(format!("{}", version), "1.0.0-beta");
}

#[test]
fn test_permission_filesystem() {
    let perm = Permission::FileSystem {
        paths: vec!["/tmp".to_string()],
        read: true,
        write: false,
    };
    let json = serde_json::to_string(&perm).unwrap();
    assert!(json.contains("file_system"));
    assert!(json.contains("/tmp"));
}

#[test]
fn test_permission_network() {
    let perm = Permission::Network {
        hosts: vec!["example.com".to_string()],
    };
    let json = serde_json::to_string(&perm).unwrap();
    assert!(json.contains("network"));
    assert!(json.contains("example.com"));
}

#[test]
fn test_permission_shell() {
    let perm = Permission::Shell {
        commands: vec!["git".to_string()],
    };
    let json = serde_json::to_string(&perm).unwrap();
    assert!(json.contains("shell"));
    assert!(json.contains("git"));
}

#[test]
fn test_permission_environment() {
    let perm = Permission::Environment {
        variables: vec!["PATH".to_string()],
    };
    let json = serde_json::to_string(&perm).unwrap();
    assert!(json.contains("environment"));
    assert!(json.contains("PATH"));
}

#[test]
fn test_risk_level_clone() {
    let level = RiskLevel::Medium;
    let cloned = level;
    assert_eq!(cloned, RiskLevel::Medium);
}

#[test]
fn test_risk_level_debug() {
    let debug = format!("{:?}", RiskLevel::High);
    assert!(debug.contains("High"));
}

#[test]
fn test_risk_level_eq() {
    assert_eq!(RiskLevel::Low, RiskLevel::Low);
    assert_ne!(RiskLevel::Low, RiskLevel::High);
}

#[test]
fn test_stop_reason_deserialization() {
    let reason: StopReason = serde_json::from_str("\"stop_sequence\"").unwrap();
    assert_eq!(reason, StopReason::StopSequence);

    let reason: StopReason = serde_json::from_str("\"tool_use\"").unwrap();
    assert_eq!(reason, StopReason::ToolUse);
}

#[test]
fn test_stop_reason_eq() {
    assert_eq!(StopReason::EndTurn, StopReason::EndTurn);
    assert_ne!(StopReason::EndTurn, StopReason::MaxTokens);
}

#[test]
fn test_stop_reason_clone() {
    let reason = StopReason::ToolUse;
    let cloned = reason;
    assert_eq!(cloned, StopReason::ToolUse);
}

#[test]
fn test_stop_reason_debug() {
    let debug = format!("{:?}", StopReason::MaxTokens);
    assert!(debug.contains("MaxTokens"));
}

#[test]
fn test_usage_clone() {
    let usage = Usage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
        cache_creation_tokens: None,
        cache_read_tokens: None,
    };
    let cloned = usage.clone();
    assert_eq!(cloned.prompt_tokens, 10);
    assert_eq!(cloned.completion_tokens, 20);
}

#[test]
fn test_usage_debug() {
    let usage = Usage::default();
    let debug = format!("{:?}", usage);
    assert!(debug.contains("Usage"));
}

#[test]
fn test_usage_deserialization() {
    let json = r#"{"prompt_tokens":100,"completion_tokens":50,"total_tokens":150}"#;
    let usage: Usage = serde_json::from_str(json).unwrap();
    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
    assert_eq!(usage.total_tokens, 150);
}

#[test]
fn test_author_clone() {
    let author = Author {
        name: "Test".to_string(),
        email: None,
        url: None,
    };
    let cloned = author.clone();
    assert_eq!(cloned.name, "Test");
}

#[test]
fn test_author_debug() {
    let author = Author {
        name: "Author".to_string(),
        email: None,
        url: None,
    };
    let debug = format!("{:?}", author);
    assert!(debug.contains("Author"));
}

#[test]
fn test_author_deserialization() {
    let json = r#"{"name":"Test Author"}"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert_eq!(author.name, "Test Author");
    assert!(author.email.is_none());
}

#[test]
fn test_version_clone() {
    let version = Version::new(1, 0, 0);
    let cloned = version.clone();
    assert_eq!(cloned.major, 1);
    assert_eq!(cloned.minor, 0);
}

#[test]
fn test_version_debug() {
    let version = Version::new(2, 1, 0);
    let debug = format!("{:?}", version);
    assert!(debug.contains("Version"));
}

#[test]
fn test_version_serialization() {
    let version = Version::new(1, 2, 3);
    let json = serde_json::to_string(&version).unwrap();
    assert!(json.contains("\"major\":1"));
    assert!(json.contains("\"minor\":2"));
    assert!(json.contains("\"patch\":3"));
}

#[test]
fn test_version_deserialization() {
    let json = r#"{"major":3,"minor":2,"patch":1}"#;
    let version: Version = serde_json::from_str(json).unwrap();
    assert_eq!(version.major, 3);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 1);
}

#[test]
fn test_version_with_prerelease() {
    let version = Version {
        major: 1,
        minor: 0,
        patch: 0,
        prerelease: Some("alpha.1".to_string()),
    };
    assert_eq!(format!("{}", version), "1.0.0-alpha.1");
}

#[test]
fn test_permission_clone() {
    let perm = Permission::Network {
        hosts: vec!["test.com".to_string()],
    };
    let cloned = perm.clone();
    match cloned {
        Permission::Network { hosts } => assert_eq!(hosts, vec!["test.com"]),
        _ => panic!("Expected Network permission"),
    }
}

#[test]
fn test_permission_debug() {
    let perm = Permission::Shell {
        commands: vec!["ls".to_string()],
    };
    let debug = format!("{:?}", perm);
    assert!(debug.contains("Shell"));
}

#[test]
fn test_permission_deserialization() {
    let json = r#"{"type":"network","hosts":["api.example.com"]}"#;
    let perm: Permission = serde_json::from_str(json).unwrap();
    match perm {
        Permission::Network { hosts } => {
            assert_eq!(hosts, vec!["api.example.com"]);
        }
        _ => panic!("Expected Network permission"),
    }
}

#[test]
fn test_risk_level_all_variants() {
    let levels = vec![RiskLevel::Low, RiskLevel::Medium, RiskLevel::High];
    for level in levels {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: RiskLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, level);
    }
}

#[test]
fn test_stop_reason_all_variants() {
    let reasons = vec![
        StopReason::EndTurn,
        StopReason::StopSequence,
        StopReason::MaxTokens,
        StopReason::ToolUse,
    ];
    for reason in reasons {
        let json = serde_json::to_string(&reason).unwrap();
        let parsed: StopReason = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, reason);
    }
}

#[test]
fn test_usage_serialization_skips_none() {
    let usage = Usage::default();
    let json = serde_json::to_string(&usage).unwrap();
    assert!(!json.contains("cache_creation_tokens"));
    assert!(!json.contains("cache_read_tokens"));
}

#[test]
fn test_author_serialization_skips_none() {
    let author = Author {
        name: "Test".to_string(),
        email: None,
        url: None,
    };
    let json = serde_json::to_string(&author).unwrap();
    assert!(!json.contains("email"));
    assert!(!json.contains("url"));
}
