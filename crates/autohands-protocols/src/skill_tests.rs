use super::*;

#[test]
fn test_skill_new() {
    let definition = SkillDefinition::new("test", "Test Skill");
    let skill = Skill::new(definition, "Content here");
    assert_eq!(skill.definition.id, "test");
    assert_eq!(skill.content, "Content here");
    assert!(skill.sections.is_empty());
}

#[test]
fn test_skill_render() {
    let definition = SkillDefinition::new("test", "Test Skill");
    let skill = Skill::new(definition, "Hello, {{name}}!");

    let mut variables = HashMap::new();
    variables.insert("name".to_string(), "World".to_string());

    let result = skill.render(&variables);
    assert_eq!(result, "Hello, World!");
}

#[test]
fn test_skill_render_multiple_variables() {
    let definition = SkillDefinition::new("test", "Test Skill");
    let skill = Skill::new(definition, "{{greeting}}, {{name}}!");

    let mut variables = HashMap::new();
    variables.insert("greeting".to_string(), "Hi".to_string());
    variables.insert("name".to_string(), "Alice".to_string());

    let result = skill.render(&variables);
    assert_eq!(result, "Hi, Alice!");
}

#[test]
fn test_skill_render_no_variables() {
    let definition = SkillDefinition::new("test", "Test Skill");
    let skill = Skill::new(definition, "Static content");

    let variables = HashMap::new();
    let result = skill.render(&variables);
    assert_eq!(result, "Static content");
}

#[test]
fn test_skill_definition_new() {
    let definition = SkillDefinition::new("test-id", "Test Name");
    assert_eq!(definition.id, "test-id");
    assert_eq!(definition.name, "Test Name");
    assert!(definition.description.is_empty());
    assert!(definition.enabled);
    assert_eq!(definition.priority, 0);
}

#[test]
fn test_skill_definition_with_description() {
    let definition = SkillDefinition::new("test", "Test")
        .with_description("A test skill");
    assert_eq!(definition.description, "A test skill");
}

#[test]
fn test_skill_definition_serialization() {
    let definition = SkillDefinition::new("test", "Test");
    let json = serde_json::to_string(&definition).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("Test"));
}

#[test]
fn test_skill_definition_deserialization() {
    let json = r#"{"id":"test","name":"Test Name","description":"","tags":[],"variables":[],"required_tools":[],"enabled":true,"priority":0,"metadata":{}}"#;
    let definition: SkillDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(definition.id, "test");
    assert!(definition.enabled);
}

#[test]
fn test_skill_variable() {
    let var = SkillVariable {
        name: "path".to_string(),
        description: "File path".to_string(),
        required: true,
        default: None,
    };
    assert_eq!(var.name, "path");
    assert!(var.required);
    assert!(var.default.is_none());
}

#[test]
fn test_skill_variable_with_default() {
    let var = SkillVariable {
        name: "timeout".to_string(),
        description: "Timeout in seconds".to_string(),
        required: false,
        default: Some("30".to_string()),
    };
    assert!(!var.required);
    assert_eq!(var.default, Some("30".to_string()));
}

#[test]
fn test_skill_clone() {
    let definition = SkillDefinition::new("test", "Test");
    let skill = Skill::new(definition, "Content");
    let cloned = skill.clone();
    assert_eq!(cloned.definition.id, skill.definition.id);
    assert_eq!(cloned.content, skill.content);
}

#[test]
fn test_default_true() {
    assert!(default_true());
}

#[test]
fn test_skill_definition_full() {
    let definition = SkillDefinition {
        id: "full-test".to_string(),
        name: "Full Test".to_string(),
        description: "A full test skill".to_string(),
        category: Some("testing".to_string()),
        tags: vec!["test".to_string(), "example".to_string()],
        variables: vec![SkillVariable {
            name: "var1".to_string(),
            description: "Variable 1".to_string(),
            required: true,
            default: None,
        }],
        required_tools: vec!["read_file".to_string()],
        enabled: true,
        priority: 10,
        metadata: HashMap::new(),
    };

    assert_eq!(definition.category, Some("testing".to_string()));
    assert_eq!(definition.tags.len(), 2);
    assert_eq!(definition.variables.len(), 1);
    assert_eq!(definition.required_tools.len(), 1);
    assert_eq!(definition.priority, 10);
}
