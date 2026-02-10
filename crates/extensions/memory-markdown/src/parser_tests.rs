    use super::*;

    #[test]
    fn test_parse_markdown() {
        let content = r#"---
id: mem_123
type: fact
tags:
  - test
  - example
importance: 0.8
created: 2024-02-07T10:30:00Z
---

# Test Memory

This is the content.
"#;

        let memory = MarkdownParser::parse(content).unwrap();
        assert_eq!(memory.front_matter.id, "mem_123");
        assert_eq!(memory.front_matter.memory_type, "fact");
        assert_eq!(memory.front_matter.tags.len(), 2);
        assert_eq!(memory.front_matter.importance, Some(0.8));
        assert!(memory.content.contains("Test Memory"));
    }

    #[test]
    fn test_markdown_memory_new() {
        let memory = MarkdownMemory::new("mem_456", "preference", "User prefers dark mode");
        assert_eq!(memory.front_matter.id, "mem_456");
        assert_eq!(memory.front_matter.memory_type, "preference");
        assert_eq!(memory.content, "User prefers dark mode");
    }

    #[test]
    fn test_markdown_memory_with_tags() {
        let memory = MarkdownMemory::new("mem_789", "fact", "Content")
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(memory.front_matter.tags.len(), 2);
    }

    #[test]
    fn test_markdown_memory_with_importance() {
        let memory = MarkdownMemory::new("mem_abc", "fact", "Content").with_importance(0.9);
        assert_eq!(memory.front_matter.importance, Some(0.9));
    }

    #[test]
    fn test_to_markdown() {
        let memory = MarkdownMemory::new("mem_test", "fact", "Test content")
            .with_tags(vec!["test".to_string()]);

        let md = memory.to_markdown().unwrap();
        assert!(md.starts_with("---"));
        assert!(md.contains("id: mem_test"));
        assert!(md.contains("type: fact"));
        assert!(md.contains("Test content"));
    }

    #[test]
    fn test_roundtrip() {
        let original = MarkdownMemory::new("mem_roundtrip", "fact", "Roundtrip test content")
            .with_tags(vec!["test".to_string()])
            .with_importance(0.5);

        let md = original.to_markdown().unwrap();
        let parsed = MarkdownParser::parse(&md).unwrap();

        assert_eq!(parsed.front_matter.id, original.front_matter.id);
        assert_eq!(parsed.front_matter.memory_type, original.front_matter.memory_type);
        assert_eq!(parsed.front_matter.tags, original.front_matter.tags);
        assert_eq!(parsed.front_matter.importance, original.front_matter.importance);
        assert_eq!(parsed.content, original.content);
    }

    #[test]
    fn test_id_to_filename() {
        assert_eq!(MarkdownParser::id_to_filename("mem_123"), "mem_123.md");
        assert_eq!(MarkdownParser::id_to_filename("test/id"), "test_id.md");
        assert_eq!(MarkdownParser::id_to_filename("a b c"), "a_b_c.md");
    }

    #[test]
    fn test_filename_to_id() {
        assert_eq!(MarkdownParser::filename_to_id("mem_123.md"), Some("mem_123".to_string()));
        assert_eq!(MarkdownParser::filename_to_id("test.txt"), None);
    }

    #[test]
    fn test_parse_missing_front_matter() {
        let content = "No front matter here";
        let result = MarkdownParser::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_front_matter() {
        let content = "---\nid: test\ntype: fact";
        let result = MarkdownParser::parse(content);
        assert!(result.is_err());
    }
