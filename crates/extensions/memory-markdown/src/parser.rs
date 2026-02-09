//! Markdown parser for memory files.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::MarkdownMemoryError;

/// Front matter structure for Markdown memory files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    /// Unique memory ID.
    pub id: String,

    /// Memory type (e.g., "fact", "preference", "conversation").
    #[serde(rename = "type")]
    pub memory_type: String,

    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Importance score (0.0 to 1.0).
    #[serde(default)]
    pub importance: Option<f32>,

    /// Creation timestamp.
    pub created: DateTime<Utc>,

    /// Last update timestamp.
    #[serde(default)]
    pub updated: Option<DateTime<Utc>>,

    /// Additional metadata.
    #[serde(default, flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Parsed Markdown memory.
#[derive(Debug, Clone)]
pub struct MarkdownMemory {
    /// Front matter metadata.
    pub front_matter: FrontMatter,

    /// Markdown content body.
    pub content: String,
}

impl MarkdownMemory {
    /// Create a new Markdown memory.
    pub fn new(id: impl Into<String>, memory_type: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            front_matter: FrontMatter {
                id: id.into(),
                memory_type: memory_type.into(),
                tags: Vec::new(),
                importance: None,
                created: Utc::now(),
                updated: None,
                metadata: HashMap::new(),
            },
            content: content.into(),
        }
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.front_matter.tags = tags;
        self
    }

    /// Set importance.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.front_matter.importance = Some(importance);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.front_matter.metadata = metadata;
        self
    }

    /// Serialize to Markdown string.
    pub fn to_markdown(&self) -> Result<String, MarkdownMemoryError> {
        let yaml = serde_yaml::to_string(&self.front_matter)
            .map_err(|e| MarkdownMemoryError::YamlSerialize(e.to_string()))?;

        Ok(format!("---\n{}---\n\n{}", yaml, self.content))
    }
}

/// Parser for Markdown memory files.
pub struct MarkdownParser;

impl MarkdownParser {
    /// Parse Markdown content into a MarkdownMemory.
    pub fn parse(content: &str) -> Result<MarkdownMemory, MarkdownMemoryError> {
        let (front_matter, body) = Self::split_front_matter(content)?;

        let fm: FrontMatter = serde_yaml::from_str(&front_matter)
            .map_err(|e| MarkdownMemoryError::YamlParse(e.to_string()))?;

        Ok(MarkdownMemory {
            front_matter: fm,
            content: body.trim().to_string(),
        })
    }

    /// Split content into front matter and body.
    fn split_front_matter(content: &str) -> Result<(String, String), MarkdownMemoryError> {
        let content = content.trim();

        if !content.starts_with("---") {
            return Err(MarkdownMemoryError::YamlParse(
                "Missing front matter delimiter".to_string(),
            ));
        }

        // Find the closing ---
        let rest = &content[3..];
        if let Some(end_pos) = rest.find("\n---") {
            let front_matter = rest[..end_pos].trim().to_string();
            let body = rest[end_pos + 4..].to_string();
            Ok((front_matter, body))
        } else {
            Err(MarkdownMemoryError::YamlParse(
                "Missing closing front matter delimiter".to_string(),
            ))
        }
    }

    /// Generate a safe filename from memory ID.
    pub fn id_to_filename(id: &str) -> String {
        // Replace any unsafe characters
        let safe_id: String = id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        format!("{}.md", safe_id)
    }

    /// Extract memory ID from filename.
    pub fn filename_to_id(filename: &str) -> Option<String> {
        filename.strip_suffix(".md").map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
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
}
