//! AutoHands native skill format adapter.
//!
//! Native format with full feature support:
//!
//! ```markdown
//! ---
//! id: code-review
//! name: Code Review Expert
//! version: 1.0.0
//! description: Expert code reviewer
//!
//! requires:
//!   tools: [read_file, glob, grep]
//!   bins: [git]
//!   any_bins: [rg, grep]
//!
//! tags: [development, review]
//! category: development
//! priority: 20
//!
//! variables:
//!   - name: focus
//!     description: Areas to focus on
//!     required: false
//!     default: "all"
//! ---
//!
//! # Code Review Expert
//!
//! You are an expert code reviewer...
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition, SkillVariable};

use super::SkillAdapter;

/// AutoHands native format adapter.
pub struct AutoHandsAdapter;

impl AutoHandsAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AutoHandsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// AutoHands frontmatter structure.
#[derive(Debug, Deserialize)]
struct AutoHandsFrontmatter {
    /// Unique skill identifier (required).
    id: String,
    /// Human-readable name.
    name: String,
    /// Skill version.
    #[serde(default)]
    version: Option<String>,
    /// Description.
    #[serde(default)]
    description: String,
    /// Category.
    #[serde(default)]
    category: Option<String>,
    /// Tags.
    #[serde(default)]
    tags: Vec<String>,
    /// Priority.
    #[serde(default)]
    priority: i32,
    /// Requirements.
    #[serde(default)]
    requires: Option<Requirements>,
    /// Variables.
    #[serde(default)]
    variables: Vec<VariableDef>,
    /// Extra fields.
    #[serde(default, flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct Requirements {
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    bins: Vec<String>,
    #[serde(default)]
    any_bins: Vec<String>,
    #[serde(default)]
    all_bins: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VariableDef {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: Option<String>,
}

impl SkillAdapter for AutoHandsAdapter {
    fn name(&self) -> &'static str {
        "autohands"
    }

    fn can_handle(&self, content: &str, file_name: &str) -> bool {
        // Check file name
        if !matches!(file_name, "SKILL.markdown" | "SKILL.md") {
            // Also support single-file .markdown files
            if !file_name.ends_with(".markdown") {
                return false;
            }
        }

        // Check for AutoHands-specific fields: `id:` is required
        if let Some((frontmatter, _)) = extract_frontmatter(content) {
            frontmatter.contains("id:")
        } else {
            false
        }
    }

    fn parse(&self, content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError> {
        let (frontmatter_str, markdown) = extract_frontmatter(content).ok_or_else(|| {
            SkillError::ParsingError("Missing YAML frontmatter".to_string())
        })?;

        let fm: AutoHandsFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| SkillError::ParsingError(format!("Invalid frontmatter: {}", e)))?;

        let mut def = SkillDefinition::new(&fm.id, &fm.name).with_description(&fm.description);

        def.category = fm.category;
        def.tags = fm.tags;
        def.priority = fm.priority;
        def.enabled = true;

        // Variables
        def.variables = fm
            .variables
            .into_iter()
            .map(|v| SkillVariable {
                name: v.name,
                description: v.description,
                required: v.required,
                default: v.default,
            })
            .collect();

        // Requirements
        if let Some(req) = fm.requires {
            def.required_tools = req.tools;

            let all_bins: Vec<String> = req
                .bins
                .into_iter()
                .chain(req.all_bins.into_iter())
                .collect();
            if !all_bins.is_empty() {
                def.metadata
                    .insert("all_bins".to_string(), serde_json::json!(all_bins));
            }
            if !req.any_bins.is_empty() {
                def.metadata
                    .insert("any_bins".to_string(), serde_json::json!(req.any_bins));
            }
        }

        // Metadata
        if let Some(version) = fm.version {
            def.metadata
                .insert("version".to_string(), serde_json::json!(version));
        }
        if let Some(dir) = base_dir {
            def.metadata.insert(
                "base_dir".to_string(),
                serde_json::json!(dir.to_string_lossy()),
            );
        }
        def.metadata
            .insert("source_format".to_string(), serde_json::json!("autohands"));

        for (k, v) in fm.extra {
            def.metadata.insert(k, v);
        }

        Ok(Skill::new(def, markdown.trim()))
    }

    fn supported_file_names(&self) -> &[&'static str] {
        &["SKILL.markdown", "SKILL.md"]
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
#[path = "autohands_tests.rs"]
mod tests;
