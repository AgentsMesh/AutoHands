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
#[path = "parser_tests.rs"]
mod tests;
