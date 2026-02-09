//! Microsoft Skills format adapter.
//!
//! Microsoft Skills use a simple format similar to Claude Code:
//!
//! ```markdown
//! ---
//! name: mcp-builder
//! description: Build MCP servers for LLM tool integration. Use when building MCP servers...
//! ---
//!
//! # MCP Builder
//!
//! Content here...
//! ```
//!
//! Key characteristics:
//! - No `id` field (derived from `name`)
//! - No `metadata` block
//! - May have `references/` subdirectory
//! - File names: `SKILL.md`, `AGENTS.md`
//!
//! Microsoft also uses a symlink structure in `skills/<language>/<category>/`
//! pointing to `.github/skills/<skill-name>/`.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition};

use super::SkillAdapter;

/// Microsoft Skills format adapter.
pub struct MicrosoftAdapter;

impl MicrosoftAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MicrosoftAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Microsoft Skills frontmatter structure.
#[derive(Debug, Deserialize)]
struct MicrosoftFrontmatter {
    /// Skill name (kebab-case, e.g., "mcp-builder").
    name: String,
    /// Description (often includes trigger conditions).
    description: String,
    /// Package name (for SDK skills).
    #[serde(default)]
    package: Option<String>,
    /// Extra fields.
    #[serde(default, flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl SkillAdapter for MicrosoftAdapter {
    fn name(&self) -> &'static str {
        "microsoft"
    }

    fn can_handle(&self, content: &str, file_name: &str) -> bool {
        // Check file name
        let valid_name = matches!(file_name, "SKILL.md" | "AGENTS.md");
        if !valid_name {
            return false;
        }

        // Microsoft format: has `name:` and `description:`, no `id:`, no `metadata:`
        if let Some((frontmatter, _)) = extract_frontmatter(content) {
            frontmatter.contains("name:")
                && frontmatter.contains("description:")
                && !frontmatter.contains("id:")
                && !frontmatter.contains("metadata:")
                && !frontmatter.contains("license:")  // Not Claude Code with license
        } else {
            false
        }
    }

    fn parse(&self, content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError> {
        let (frontmatter_str, markdown) = extract_frontmatter(content).ok_or_else(|| {
            SkillError::ParsingError("Missing YAML frontmatter".to_string())
        })?;

        let fm: MicrosoftFrontmatter = serde_yaml::from_str(&frontmatter_str)
            .map_err(|e| SkillError::ParsingError(format!("Invalid frontmatter: {}", e)))?;

        // Microsoft uses name directly as ID (already kebab-case)
        let id = fm.name.clone();

        let mut def = SkillDefinition::new(&id, &fm.name).with_description(&fm.description);
        def.enabled = true;

        // Infer category from skill name suffix (e.g., "-py", "-ts", "-dotnet")
        def.category = infer_category(&fm.name);

        // Infer tags from description and name
        def.tags = infer_tags(&fm.name, &fm.description);

        // Package metadata
        if let Some(package) = fm.package {
            def.metadata
                .insert("package".to_string(), serde_json::json!(package));
        }

        // Check for references/ directory
        if let Some(dir) = base_dir {
            let refs_dir = dir.join("references");
            if refs_dir.exists() {
                def.metadata
                    .insert("has_references".to_string(), serde_json::json!(true));
            }

            let scripts_dir = dir.join("scripts");
            if scripts_dir.exists() {
                def.metadata
                    .insert("has_scripts".to_string(), serde_json::json!(true));
            }

            def.metadata.insert(
                "base_dir".to_string(),
                serde_json::json!(dir.to_string_lossy()),
            );
        }

        def.metadata
            .insert("source_format".to_string(), serde_json::json!("microsoft"));

        for (k, v) in fm.extra {
            def.metadata.insert(k, v);
        }

        Ok(Skill::new(def, markdown.trim()))
    }

    fn supported_file_names(&self) -> &[&'static str] {
        &["SKILL.md", "AGENTS.md"]
    }
}

/// Infer category from skill name.
fn infer_category(name: &str) -> Option<String> {
    // Check language suffix
    if name.ends_with("-py") {
        return Some("python".to_string());
    }
    if name.ends_with("-ts") || name.ends_with("-js") {
        return Some("typescript".to_string());
    }
    if name.ends_with("-dotnet") {
        return Some("dotnet".to_string());
    }
    if name.ends_with("-java") {
        return Some("java".to_string());
    }
    if name.ends_with("-rust") {
        return Some("rust".to_string());
    }

    // Check common prefixes
    if name.starts_with("azure-") {
        return Some("azure".to_string());
    }
    if name.starts_with("mcp-") {
        return Some("mcp".to_string());
    }

    Some("development".to_string())
}

/// Infer tags from name and description.
fn infer_tags(name: &str, description: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let desc_lower = description.to_lowercase();

    // Language tags
    if name.ends_with("-py") || desc_lower.contains("python") {
        tags.push("python".to_string());
    }
    if name.ends_with("-ts") || desc_lower.contains("typescript") {
        tags.push("typescript".to_string());
    }
    if name.ends_with("-dotnet") || desc_lower.contains(".net") || desc_lower.contains("c#") {
        tags.push("dotnet".to_string());
    }
    if name.ends_with("-java") || desc_lower.contains("java") {
        tags.push("java".to_string());
    }
    if name.ends_with("-rust") || desc_lower.contains("rust") {
        tags.push("rust".to_string());
    }

    // Feature tags
    if desc_lower.contains("azure") {
        tags.push("azure".to_string());
    }
    if desc_lower.contains("mcp") || desc_lower.contains("model context protocol") {
        tags.push("mcp".to_string());
    }
    if desc_lower.contains("api") {
        tags.push("api".to_string());
    }
    if desc_lower.contains("database") || desc_lower.contains("cosmos") || desc_lower.contains("sql") {
        tags.push("database".to_string());
    }
    if desc_lower.contains("auth") || desc_lower.contains("identity") {
        tags.push("auth".to_string());
    }

    tags
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
mod tests {
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
}
