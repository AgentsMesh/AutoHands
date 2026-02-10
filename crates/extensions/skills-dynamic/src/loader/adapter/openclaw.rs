//! OpenClaw skill format adapter.
//!
//! OpenClaw uses a format similar to Claude Code but with nested `metadata.openclaw`:
//!
//! ```markdown
//! ---
//! name: wechat-publisher
//! description: "一键发布 Markdown 到微信公众号草稿箱"
//! metadata:
//!   openclaw:
//!     emoji: "📱"
//! ---
//!
//! # WeChat Publisher
//!
//! Content here...
//! ```
//!
//! Additionally, OpenClaw skills have a `_meta.json` file with:
//! - owner: skill creator ID
//! - slug: URL-friendly identifier
//! - displayName: human-readable name
//! - latest: version info with commit reference
//!
//! Key characteristics:
//! - No `id` field (uses `name` as ID)
//! - `metadata.openclaw` with emoji and other config
//! - File names: `SKILL.md`, `skill.md`

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition};

use super::SkillAdapter;

/// OpenClaw format adapter.
pub struct OpenClawAdapter;

impl OpenClawAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenClawAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenClaw frontmatter structure.
#[derive(Debug, Deserialize)]
struct OpenClawFrontmatter {
    /// Skill name.
    name: String,
    /// Description.
    #[serde(default)]
    description: String,
    /// OpenClaw metadata.
    #[serde(default)]
    metadata: Option<OpenClawMetadataWrapper>,
    /// Extra fields.
    #[serde(default, flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenClawMetadataWrapper {
    #[serde(default)]
    openclaw: Option<OpenClawConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenClawConfig {
    #[serde(default)]
    emoji: Option<String>,
    #[serde(default, flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// OpenClaw _meta.json structure.
#[derive(Debug, Deserialize)]
pub(crate) struct OpenClawMetaJson {
    pub(crate) owner: String,
    pub(crate) slug: String,
    #[serde(rename = "displayName")]
    pub(crate) display_name: String,
    pub(crate) latest: OpenClawVersion,
    #[serde(default, rename = "history")]
    pub(crate) _history: Vec<OpenClawVersion>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenClawVersion {
    pub(crate) version: String,
    #[serde(rename = "publishedAt")]
    pub(crate) _published_at: u64,
    pub(crate) commit: String,
}

impl SkillAdapter for OpenClawAdapter {
    fn name(&self) -> &'static str {
        "openclaw"
    }

    fn can_handle(&self, content: &str, file_name: &str) -> bool {
        // Check file name
        let valid_name = matches!(file_name, "SKILL.md" | "skill.md");
        if !valid_name {
            return false;
        }

        // Check for OpenClaw pattern: has `metadata:` with nested `openclaw:`
        if let Some((frontmatter, _)) = extract_frontmatter(content) {
            frontmatter.contains("metadata:") && frontmatter.contains("openclaw")
        } else {
            false
        }
    }

    fn parse(&self, content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError> {
        let (frontmatter_str, markdown) = extract_frontmatter(content).ok_or_else(|| {
            SkillError::ParsingError("Missing YAML frontmatter".to_string())
        })?;

        let fm: OpenClawFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| SkillError::ParsingError(format!("Invalid frontmatter: {}", e)))?;

        // Use name as ID
        let id = name_to_id(&fm.name);

        let mut def = SkillDefinition::new(&id, &fm.name).with_description(&fm.description);
        def.enabled = true;

        // Try to load _meta.json for additional info
        if let Some(dir) = base_dir {
            if let Ok(meta) = load_meta_json(dir) {
                // Use displayName if available
                def.name = meta.display_name;
                def.metadata
                    .insert("owner".to_string(), serde_json::json!(meta.owner));
                def.metadata
                    .insert("slug".to_string(), serde_json::json!(meta.slug));
                def.metadata
                    .insert("version".to_string(), serde_json::json!(meta.latest.version));
                def.metadata
                    .insert("commit".to_string(), serde_json::json!(meta.latest.commit));
            }
        }

        // Extract OpenClaw metadata
        if let Some(metadata) = fm.metadata {
            if let Some(openclaw) = metadata.openclaw {
                if let Some(emoji) = openclaw.emoji {
                    def.metadata
                        .insert("emoji".to_string(), serde_json::json!(emoji));
                }
                for (k, v) in openclaw.extra {
                    def.metadata.insert(format!("openclaw_{}", k), v);
                }
            }
        }

        // Base dir and format
        if let Some(dir) = base_dir {
            def.metadata.insert(
                "base_dir".to_string(),
                serde_json::json!(dir.to_string_lossy()),
            );
        }
        def.metadata
            .insert("source_format".to_string(), serde_json::json!("openclaw"));

        for (k, v) in fm.extra {
            def.metadata.insert(k, v);
        }

        Ok(Skill::new(def, markdown.trim()))
    }

    fn supported_file_names(&self) -> &[&'static str] {
        &["SKILL.md", "skill.md"]
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

/// Load _meta.json from directory.
fn load_meta_json(dir: &Path) -> Result<OpenClawMetaJson, SkillError> {
    let meta_path = dir.join("_meta.json");
    let content = std::fs::read_to_string(&meta_path).map_err(|e| {
        SkillError::LoadingFailed(format!("Failed to read _meta.json: {}", e))
    })?;

    serde_json::from_str(&content)
        .map_err(|e| SkillError::ParsingError(format!("Invalid _meta.json: {}", e)))
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
#[path = "openclaw_tests.rs"]
mod tests;
