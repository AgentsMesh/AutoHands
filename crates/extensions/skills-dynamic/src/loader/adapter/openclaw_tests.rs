    use super::*;

    const SAMPLE: &str = r#"---
name: wechat-publisher
description: "一键发布 Markdown 到微信公众号草稿箱"
metadata:
  openclaw:
    emoji: "📱"
---

# WeChat Publisher

Content here.
"#;

    #[test]
    fn test_can_handle() {
        let adapter = OpenClawAdapter::new();
        assert!(adapter.can_handle(SAMPLE, "SKILL.md"));
        assert!(adapter.can_handle(SAMPLE, "skill.md"));

        // Should not handle Claude Code format (no metadata.openclaw)
        let claude = "---\nname: test\ndescription: Test\n---\nContent";
        assert!(!adapter.can_handle(claude, "SKILL.md"));
    }

    #[test]
    fn test_parse() {
        let adapter = OpenClawAdapter::new();
        let skill = adapter.parse(SAMPLE, None).unwrap();

        assert_eq!(skill.definition.id, "wechat-publisher");
        assert_eq!(skill.definition.name, "wechat-publisher");
        assert_eq!(
            skill.definition.metadata.get("emoji"),
            Some(&serde_json::json!("📱"))
        );
        assert_eq!(
            skill.definition.metadata.get("source_format"),
            Some(&serde_json::json!("openclaw"))
        );
    }
