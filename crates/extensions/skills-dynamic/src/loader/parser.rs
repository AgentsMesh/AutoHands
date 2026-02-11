//! SKILL.markdown parser.
//!
//! Legacy parser for backward compatibility.
//! New code should use the adapter system in `loader/adapter/`.
//!
//! This parser handles the AutoHands native format:
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
//!
//! tags: [development, review]
//! ---
//!
//! # Code Review Expert
//!
//! You are an expert code reviewer...
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition, SkillVariable};

/// YAML frontmatter structure (AutoHands native format).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillFrontmatter {
    /// Unique skill identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Skill version (semver).
    #[serde(default)]
    pub version: Option<String>,

    /// Description of the skill.
    #[serde(default)]
    pub description: String,

    /// Category for organization.
    #[serde(default)]
    pub category: Option<String>,

    /// Tags for discovery.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Skill priority (higher = more preferred).
    #[serde(default)]
    pub priority: i32,

    /// Skill requirements (dependencies).
    #[serde(default)]
    pub requires: Option<SkillRequirements>,

    /// Variable definitions.
    #[serde(default)]
    pub variables: Vec<SkillVariableDef>,

    /// Additional metadata.
    #[serde(default, flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Skill requirements for dependency checking.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillRequirements {
    /// Required tools (all must be available).
    #[serde(default)]
    pub tools: Vec<String>,

    /// Required binaries (all must be available).
    #[serde(default)]
    pub bins: Vec<String>,

    /// Any of these binaries must be available.
    #[serde(default)]
    pub any_bins: Vec<String>,

    /// All of these binaries must be available.
    #[serde(default)]
    pub all_bins: Vec<String>,
}

/// Variable definition in frontmatter.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillVariableDef {
    /// Variable name.
    pub name: String,

    /// Variable description.
    #[serde(default)]
    pub description: String,

    /// Whether the variable is required.
    #[serde(default)]
    pub required: bool,

    /// Default value.
    #[serde(default)]
    pub default: Option<String>,
}

/// Skill metadata for extended functionality.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillMetadata {
    /// Any of these binaries must be available.
    #[serde(default)]
    pub any_bins: Vec<String>,

    /// All of these binaries must be available.
    #[serde(default)]
    pub all_bins: Vec<String>,
}

/// Parse a SKILL.markdown file into a Skill.
///
/// This is the legacy parser for AutoHands native format.
/// For multi-format support, use `AdapterRegistry.parse()` instead.
///
/// # Arguments
///
/// * `content` - The raw content of the markdown file
/// * `base_dir` - Optional base directory for relative path resolution
///
/// # Returns
///
/// A parsed `Skill` or a `SkillError`.
pub fn parse_skill_markdown(content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError> {
    // Split frontmatter and content
    let (frontmatter_str, markdown_content) = extract_frontmatter(content)?;

    // Parse YAML frontmatter
    let frontmatter: SkillFrontmatter = serde_yml::from_str(&frontmatter_str)
        .map_err(|e| SkillError::ParsingError(format!("Failed to parse frontmatter: {}", e)))?;

    // Build skill definition
    let mut definition = SkillDefinition::new(&frontmatter.id, &frontmatter.name)
        .with_description(&frontmatter.description);

    definition.category = frontmatter.category;
    definition.tags = frontmatter.tags;
    definition.priority = frontmatter.priority;

    // Convert variables
    definition.variables = frontmatter
        .variables
        .into_iter()
        .map(|v| SkillVariable {
            name: v.name,
            description: v.description,
            required: v.required,
            default: v.default,
        })
        .collect();

    // Extract requirements
    if let Some(requires) = &frontmatter.requires {
        definition.required_tools = requires.tools.clone();

        // Store binary requirements in metadata
        if !requires.bins.is_empty() || !requires.all_bins.is_empty() {
            let all_bins: Vec<String> = requires
                .bins
                .iter()
                .chain(requires.all_bins.iter())
                .cloned()
                .collect();
            definition
                .metadata
                .insert("all_bins".to_string(), serde_json::json!(all_bins));
        }

        if !requires.any_bins.is_empty() {
            definition
                .metadata
                .insert("any_bins".to_string(), serde_json::json!(requires.any_bins));
        }
    }

    // Store base directory in metadata if provided
    if let Some(dir) = base_dir {
        definition.metadata.insert(
            "base_dir".to_string(),
            serde_json::json!(dir.to_string_lossy()),
        );
    }

    // Store version in metadata
    if let Some(version) = frontmatter.version {
        definition
            .metadata
            .insert("version".to_string(), serde_json::json!(version));
    }

    // Copy extra metadata
    for (key, value) in frontmatter.extra {
        definition.metadata.insert(key, value);
    }

    Ok(Skill::new(definition, markdown_content.trim()))
}

/// Extract YAML frontmatter from markdown content.
fn extract_frontmatter(content: &str) -> Result<(String, String), SkillError> {
    let content = content.trim();

    // Check for YAML frontmatter delimiter
    if !content.starts_with("---") {
        return Err(SkillError::ParsingError(
            "SKILL.markdown must start with YAML frontmatter (---)".to_string(),
        ));
    }

    // Find the closing delimiter
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        SkillError::ParsingError("Missing closing frontmatter delimiter (---)".to_string())
    })?;

    let frontmatter = after_first[..end_pos].trim().to_string();
    let markdown = after_first[end_pos + 4..].trim().to_string();

    Ok((frontmatter, markdown))
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
