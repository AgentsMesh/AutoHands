    use super::*;

    const SAMPLE: &str = r#"---
name: mcp-builder
description: Build MCP servers for LLM tool integration. Use when building MCP servers to integrate external APIs.
---

# MCP Builder

Content here.
"#;

    const SAMPLE_PY: &str = r#"---
name: azure-cosmos-py
description: Azure Cosmos DB SDK for Python. Document CRUD, queries, and containers.
package: azure-cosmos
---

# Azure Cosmos DB (Python)

Content here.
"#;

    #[test]
    fn test_can_handle() {
        let adapter = MicrosoftAdapter::new();
        assert!(adapter.can_handle(SAMPLE, "SKILL.md"));
        assert!(adapter.can_handle(SAMPLE, "AGENTS.md"));

        // Should not handle OpenClaw format (has metadata)
        let openclaw = "---\nname: test\nmetadata:\n  openclaw:\n    emoji: x\n---\nContent";
        assert!(!adapter.can_handle(openclaw, "SKILL.md"));
    }

    #[test]
    fn test_parse() {
        let adapter = MicrosoftAdapter::new();
        let skill = adapter.parse(SAMPLE, None).unwrap();

        assert_eq!(skill.definition.id, "mcp-builder");
        assert_eq!(skill.definition.name, "mcp-builder");
        assert!(skill.definition.tags.contains(&"mcp".to_string()));
    }

    #[test]
    fn test_parse_with_language_suffix() {
        let adapter = MicrosoftAdapter::new();
        let skill = adapter.parse(SAMPLE_PY, None).unwrap();

        assert_eq!(skill.definition.id, "azure-cosmos-py");
        assert_eq!(skill.definition.category, Some("python".to_string()));
        assert!(skill.definition.tags.contains(&"python".to_string()));
        assert!(skill.definition.tags.contains(&"azure".to_string()));
        assert!(skill.definition.tags.contains(&"database".to_string()));
    }

    #[test]
    fn test_infer_category() {
        assert_eq!(infer_category("azure-cosmos-py"), Some("python".to_string()));
        assert_eq!(infer_category("azure-cosmos-ts"), Some("typescript".to_string()));
        assert_eq!(infer_category("azure-cosmos-dotnet"), Some("dotnet".to_string()));
        assert_eq!(infer_category("mcp-builder"), Some("mcp".to_string()));
    }
