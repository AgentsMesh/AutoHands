    use super::*;

    const SAMPLE: &str = r#"---
id: test-skill
name: Test Skill
version: 1.0.0
description: A test skill

requires:
  tools: [read_file]
  bins: [git]

tags: [test]
---

# Test Skill

Content here.
"#;

    #[test]
    fn test_can_handle() {
        let adapter = AutoHandsAdapter::new();
        assert!(adapter.can_handle(SAMPLE, "SKILL.markdown"));
        assert!(adapter.can_handle(SAMPLE, "SKILL.md"));
        assert!(!adapter.can_handle("name: test", "SKILL.md")); // No id field
    }

    #[test]
    fn test_parse() {
        let adapter = AutoHandsAdapter::new();
        let skill = adapter.parse(SAMPLE, None).unwrap();
        assert_eq!(skill.definition.id, "test-skill");
        assert_eq!(skill.definition.name, "Test Skill");
        assert_eq!(skill.definition.required_tools, vec!["read_file"]);
    }
