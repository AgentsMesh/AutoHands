    use super::*;

    const SAMPLE: &str = r#"---
name: frontend-design
description: Create distinctive, production-grade frontend interfaces with high design quality.
license: Complete terms in LICENSE.txt
---

# Frontend Design

You are an expert frontend designer.
"#;

    #[test]
    fn test_can_handle() {
        let adapter = ClaudeCodeAdapter::new();
        assert!(adapter.can_handle(SAMPLE, "SKILL.md"));
        assert!(adapter.can_handle(SAMPLE, "CLAUDE.md"));
        assert!(!adapter.can_handle(SAMPLE, "skill.md")); // Wrong case

        // Should not handle AutoHands format
        let autohands = "---\nid: test\nname: Test\n---\nContent";
        assert!(!adapter.can_handle(autohands, "SKILL.md"));
    }

    #[test]
    fn test_parse() {
        let adapter = ClaudeCodeAdapter::new();
        let skill = adapter.parse(SAMPLE, None).unwrap();

        assert_eq!(skill.definition.id, "frontend-design");
        assert_eq!(skill.definition.name, "frontend-design");
        assert!(skill.definition.description.contains("production-grade"));
        assert_eq!(
            skill.definition.metadata.get("source_format"),
            Some(&serde_json::json!("claude-code"))
        );
    }

    #[test]
    fn test_name_to_id() {
        assert_eq!(name_to_id("Frontend Design"), "frontend-design");
        assert_eq!(name_to_id("MCP Builder"), "mcp-builder");
        assert_eq!(name_to_id("my-skill"), "my-skill");
    }
