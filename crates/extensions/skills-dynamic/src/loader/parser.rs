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
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(&frontmatter_str)
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

/// Parse multiple skill files from a directory.
#[allow(dead_code)]
pub async fn parse_skill_directory(
    dir: &Path,
) -> Result<Vec<Skill>, SkillError> {
    let mut skills = Vec::new();

    if !dir.exists() {
        return Ok(skills);
    }

    // Check for single-file skills (*.markdown)
    for entry in std::fs::read_dir(dir).map_err(|e| {
        SkillError::LoadingFailed(format!("Failed to read directory {}: {}", dir.display(), e))
    })? {
        let entry = entry.map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();

        if path.is_file() {
            // Single-file skill: name.markdown
            if let Some(ext) = path.extension() {
                if ext == "markdown" || ext == "md" {
                    if let Ok(skill) = load_single_file_skill(&path) {
                        skills.push(skill);
                    }
                }
            }
        } else if path.is_dir() {
            // Directory skill: name/SKILL.markdown
            let skill_file = path.join("SKILL.markdown");
            if skill_file.exists() {
                if let Ok(skill) = load_directory_skill(&path, &skill_file) {
                    skills.push(skill);
                }
            } else {
                // Also check for SKILL.md
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(skill) = load_directory_skill(&path, &skill_file) {
                        skills.push(skill);
                    }
                }
            }
        }
    }

    Ok(skills)
}

#[allow(dead_code)]
fn load_single_file_skill(path: &Path) -> Result<Skill, SkillError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        SkillError::LoadingFailed(format!("Failed to read skill file {}: {}", path.display(), e))
    })?;

    parse_skill_markdown(&content, path.parent())
}

#[allow(dead_code)]
fn load_directory_skill(dir: &Path, skill_file: &Path) -> Result<Skill, SkillError> {
    let content = std::fs::read_to_string(skill_file).map_err(|e| {
        SkillError::LoadingFailed(format!(
            "Failed to read skill file {}: {}",
            skill_file.display(),
            e
        ))
    })?;

    parse_skill_markdown(&content, Some(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SKILL: &str = r#"---
id: test-skill
name: Test Skill
version: 1.0.0
description: A test skill for unit testing

requires:
  tools: [read_file, write_file]
  bins: [git]

tags: [test, example]
priority: 10

variables:
  - name: target
    description: Target file or directory
    required: true
  - name: verbose
    description: Enable verbose output
    default: "false"
---

# Test Skill

You are a test assistant.

## Instructions

1. Read the target file
2. Process it
3. Write the result
"#;

    #[test]
    fn test_parse_skill_markdown() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        assert_eq!(skill.definition.id, "test-skill");
        assert_eq!(skill.definition.name, "Test Skill");
        assert_eq!(skill.definition.description, "A test skill for unit testing");
        assert_eq!(skill.definition.tags, vec!["test", "example"]);
        assert_eq!(skill.definition.priority, 10);
        assert_eq!(skill.definition.required_tools, vec!["read_file", "write_file"]);
        assert_eq!(skill.definition.variables.len(), 2);

        // Check content
        assert!(skill.content.contains("You are a test assistant"));
        assert!(skill.content.contains("## Instructions"));
    }

    #[test]
    fn test_parse_frontmatter_extraction() {
        let (frontmatter, content) = extract_frontmatter(SAMPLE_SKILL).unwrap();

        assert!(frontmatter.contains("id: test-skill"));
        assert!(content.contains("# Test Skill"));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let result = parse_skill_markdown("# No frontmatter", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let result = parse_skill_markdown("---\nid: test\nname: Test\n# Content", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_minimal_skill() {
        let minimal = r#"---
id: minimal
name: Minimal Skill
---

Simple content.
"#;
        let skill = parse_skill_markdown(minimal, None).unwrap();
        assert_eq!(skill.definition.id, "minimal");
        assert_eq!(skill.content, "Simple content.");
    }

    #[test]
    fn test_variables_conversion() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        let target_var = skill
            .definition
            .variables
            .iter()
            .find(|v| v.name == "target")
            .unwrap();
        assert!(target_var.required);
        assert!(target_var.default.is_none());

        let verbose_var = skill
            .definition
            .variables
            .iter()
            .find(|v| v.name == "verbose")
            .unwrap();
        assert!(!verbose_var.required);
        assert_eq!(verbose_var.default, Some("false".to_string()));
    }

    #[test]
    fn test_requirements_to_metadata() {
        let skill = parse_skill_markdown(SAMPLE_SKILL, None).unwrap();

        // Check that bins are stored in metadata
        let all_bins = skill.definition.metadata.get("all_bins").unwrap();
        let bins: Vec<String> = serde_json::from_value(all_bins.clone()).unwrap();
        assert!(bins.contains(&"git".to_string()));
    }
}
