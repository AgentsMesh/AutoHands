    use super::*;

    const SAMPLE_SKILL: &str = r#"---
id: test-skill
name: Test Skill
version: 1.0.0
description: A test skill for unit testing

requires:
  tools: [read_file, write_file]
  bins: [git]

tags: [test, example]
priority: 10

variables:
  - name: target
    description: Target file or directory
    required: true
  - name: verbose
    description: Enable verbose output
    default: "false"
---

# Test Skill

You are a test assistant.

## Instructions

1. Read the target file
2. Process it
3. Write the result
"#;

    #[test]
    fn test_parse_skill_markdown() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        assert_eq!(skill.definition.id, "test-skill");
        assert_eq!(skill.definition.name, "Test Skill");
        assert_eq!(skill.definition.description, "A test skill for unit testing");
        assert_eq!(skill.definition.tags, vec!["test", "example"]);
        assert_eq!(skill.definition.priority, 10);
        assert_eq!(skill.definition.required_tools, vec!["read_file", "write_file"]);
        assert_eq!(skill.definition.variables.len(), 2);

        // Check content
        assert!(skill.content.contains("You are a test assistant"));
        assert!(skill.content.contains("## Instructions"));
    }

    #[test]
    fn test_parse_frontmatter_extraction() {
        let (frontmatter, content) = extract_frontmatter(SAMPLE_SKILL).unwrap();

        assert!(frontmatter.contains("id: test-skill"));
        assert!(content.contains("# Test Skill"));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let result = parse_skill_markdown("# No frontmatter", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let result = parse_skill_markdown("---\nid: test\nname: Test\n# Content", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_minimal_skill() {
        let minimal = r#"---
id: minimal
name: Minimal Skill
---

Simple content.
"#;
        let skill = parse_skill_markdown(minimal, None).unwrap();
        assert_eq!(skill.definition.id, "minimal");
        assert_eq!(skill.content, "Simple content.");
    }

    #[test]
    fn test_variables_conversion() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        let target_var = skill
            .definition
            .variables
            .iter()
            .find(|v| v.name == "target")
            .unwrap();
        assert!(target_var.required);
        assert!(target_var.default.is_none());

        let verbose_var = skill
            .definition
            .variables
            .iter()
            .find(|v| v.name == "verbose")
            .unwrap();
        assert!(!verbose_var.required);
        assert_eq!(verbose_var.default, Some("false".to_string()));
    }

    #[test]
    fn test_requirements_to_metadata() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        // Check that bins are stored in metadata
        let all_bins = skill.definition.metadata.get("all_bins").unwrap();
        let bins: Vec<String> = serde_json::from_value(all_bins.clone()).unwrap();
        assert!(bins.contains(&"git".to_string()));
    }
