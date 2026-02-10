//! Claude Code skill format adapter.
//!
//! Claude Code uses a simple format with `name` and `description` fields:
//!
//! ```markdown
//! ---
//! name: frontend-design
//! description: Create distinctive, production-grade frontend interfaces...
//! license: Complete terms in LICENSE.txt
//! ---
//!
//! # Frontend Design
//!
//! You are an expert frontend designer...
//! ```
//!
//! Key characteristics:
//! - No `id` field (derived from `name`)
//! - Optional `license` field
//! - File names: `SKILL.md`, `CLAUDE.md`

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition};

use super::SkillAdapter;

/// Claude Code format adapter.
pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Claude Code frontmatter structure.
#[derive(Debug, Deserialize)]
struct ClaudeCodeFrontmatter {
    /// Skill name (required).
    name: String,
    /// Description (required, often includes trigger conditions).
    description: String,
    /// License reference.
    #[serde(default)]
    license: Option<String>,
    /// Extra fields.
    #[serde(default, flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl SkillAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn can_handle(&self, content: &str, file_name: &str) -> bool {
        // Check file name
        let valid_name = matches!(file_name, "SKILL.md" | "CLAUDE.md");
        if !valid_name {
            return false;
        }

        // Check for Claude Code pattern: has `name:` and `description:`, but no `id:`
        if let Some((frontmatter, _)) = extract_frontmatter(content) {
            frontmatter.contains("name:")
                && frontmatter.contains("description:")
                && !frontmatter.contains("id:")
                && !frontmatter.contains("metadata:")  // Not OpenClaw
        } else {
            false
        }
    }

    fn parse(&self, content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError> {
        let (frontmatter_str, markdown) = extract_frontmatter(content).ok_or_else(|| {
            SkillError::ParsingError("Missing YAML frontmatter".to_string())
        })?;

        let fm: ClaudeCodeFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| SkillError::ParsingError(format!("Invalid frontmatter: {}", e)))?;

        // Derive ID from name
        let id = name_to_id(&fm.name);

        let mut def = SkillDefinition::new(&id, &fm.name).with_description(&fm.description);
        def.enabled = true;

        // Extract category from description if possible
        def.category = extract_category_from_description(&fm.description);

        // Metadata
        if let Some(license) = fm.license {
            def.metadata
                .insert("license".to_string(), serde_json::json!(license));
        }
        if let Some(dir) = base_dir {
            def.metadata.insert(
                "base_dir".to_string(),
                serde_json::json!(dir.to_string_lossy()),
            );
        }
        def.metadata
            .insert("source_format".to_string(), serde_json::json!("claude-code"));

        for (k, v) in fm.extra {
            def.metadata.insert(k, v);
        }

        Ok(Skill::new(def, markdown.trim()))
    }

    fn supported_file_names(&self) -> &[&'static str] {
        &["SKILL.md", "CLAUDE.md"]
    }
}

/// Convert name to kebab-case ID.
fn name_to_id(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Try to extract category from description.
fn extract_category_from_description(desc: &str) -> Option<String> {
    let desc_lower = desc.to_lowercase();

    if desc_lower.contains("frontend") || desc_lower.contains("ui") || desc_lower.contains("design") {
        Some("frontend".to_string())
    } else if desc_lower.contains("backend") || desc_lower.contains("api") || desc_lower.contains("server") {
        Some("backend".to_string())
    } else if desc_lower.contains("database") || desc_lower.contains("sql") {
        Some("database".to_string())
    } else if desc_lower.contains("devops") || desc_lower.contains("deploy") || desc_lower.contains("docker") {
        Some("devops".to_string())
    } else if desc_lower.contains("test") {
        Some("testing".to_string())
    } else if desc_lower.contains("security") {
        Some("security".to_string())
    } else {
        Some("development".to_string())
    }
}

/// Extract YAML frontmatter from content.
fn extract_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("\n---")?;

    let frontmatter = after_first[..end_pos].trim().to_string();
    let markdown = after_first[end_pos + 4..].trim().to_string();

    Some((frontmatter, markdown))
}

#[cfg(test)]
#[path = "claude_code_tests.rs"]
mod tests;
