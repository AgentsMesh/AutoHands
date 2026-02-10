use super::*;

#[test]
fn test_extension_manifest_new() {
    let manifest = ExtensionManifest::new("test-ext", "Test Extension", Version::new(1, 0, 0));
    assert_eq!(manifest.id, "test-ext");
    assert_eq!(manifest.name, "Test Extension");
    assert_eq!(manifest.version.major, 1);
    assert!(manifest.description.is_empty());
}

#[test]
fn test_extension_manifest_with_description() {
    let manifest = ExtensionManifest::new("test", "Test", Version::new(1, 0, 0))
        .with_description("A test extension");
    assert_eq!(manifest.description, "A test extension");
}

#[test]
fn test_extension_manifest_with_author() {
    let author = Author {
        name: "Test Author".to_string(),
        email: Some("test@example.com".to_string()),
        url: None,
    };
    let manifest = ExtensionManifest::new("test", "Test", Version::new(1, 0, 0))
        .with_author(author);
    assert!(manifest.author.is_some());
    assert_eq!(manifest.author.unwrap().name, "Test Author");
}

#[test]
fn test_extension_manifest_serialization() {
    let manifest = ExtensionManifest::new("test", "Test", Version::new(1, 0, 0));
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("Test"));
}

#[test]
fn test_extension_manifest_clone() {
    let manifest = ExtensionManifest::new("test", "Test", Version::new(1, 0, 0));
    let cloned = manifest.clone();
    assert_eq!(cloned.id, manifest.id);
}

#[test]
fn test_dependencies_default() {
    let deps = Dependencies::default();
    assert!(deps.required.is_empty());
    assert!(deps.optional.is_empty());
}

#[test]
fn test_dependency_spec() {
    let spec = DependencySpec {
        id: "other-ext".to_string(),
        version: Some(">=1.0.0".to_string()),
    };
    assert_eq!(spec.id, "other-ext");
    assert!(spec.version.is_some());
}

#[test]
fn test_dependency_spec_serialization() {
    let spec = DependencySpec {
        id: "dep".to_string(),
        version: None,
    };
    let json = serde_json::to_string(&spec).unwrap();
    assert!(json.contains("dep"));
}

#[test]
fn test_provides_default() {
    let provides = Provides::default();
    assert!(provides.tools.is_empty());
    assert!(provides.providers.is_empty());
    assert!(provides.channels.is_empty());
    assert!(provides.memory_backends.is_empty());
    assert!(provides.agents.is_empty());
    assert!(provides.skills.is_empty());
}

#[test]
fn test_provides_serialization() {
    let provides = Provides {
        tools: vec!["tool1".to_string()],
        providers: vec!["provider1".to_string()],
        ..Default::default()
    };
    let json = serde_json::to_string(&provides).unwrap();
    assert!(json.contains("tool1"));
    assert!(json.contains("provider1"));
}

#[test]
fn test_extension_manifest_full() {
    let manifest = ExtensionManifest {
        id: "full-ext".to_string(),
        name: "Full Extension".to_string(),
        version: Version::new(2, 1, 0),
        description: "A fully configured extension".to_string(),
        author: Some(Author {
            name: "Developer".to_string(),
            email: None,
            url: None,
        }),
        dependencies: Dependencies {
            required: vec![DependencySpec {
                id: "base".to_string(),
                version: None,
            }],
            optional: vec![],
        },
        provides: Provides {
            tools: vec!["tool1".to_string(), "tool2".to_string()],
            ..Default::default()
        },
        config_schema: Some(serde_json::json!({"type": "object"})),
        permissions: vec![],
        metadata: HashMap::new(),
    };
    assert_eq!(manifest.dependencies.required.len(), 1);
    assert_eq!(manifest.provides.tools.len(), 2);
    assert!(manifest.config_schema.is_some());
}
