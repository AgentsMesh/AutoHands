//! Filesystem-based skill loader.
//!
//! Loads skills from directories containing skill definition files.
//! Uses adapters to support multiple formats:
//!
//! - AutoHands: `SKILL.markdown`, `SKILL.md`
//! - Claude Code: `SKILL.md`, `CLAUDE.md`
//! - OpenClaw: `SKILL.md`, `skill.md`
//! - Microsoft: `SKILL.md`, `AGENTS.md`

use std::path::Path;

use tracing::{debug, warn};
use walkdir::WalkDir;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::Skill;

use super::adapter::AdapterRegistry;
use super::parser::parse_skill_markdown;

/// Filesystem skill loader with multi-format support.
#[derive(Debug, Clone)]
pub struct FilesystemLoader {
    /// Maximum directory depth for skill discovery.
    max_depth: usize,
    /// Supported file names (from adapters).
    supported_files: Vec<&'static str>,
}

impl FilesystemLoader {
    /// Create a new filesystem loader.
    pub fn new() -> Self {
        let registry = AdapterRegistry::new();
        Self {
            max_depth: 2,
            supported_files: registry.supported_file_names(),
        }
    }

    /// Set the maximum directory depth for skill discovery.
    #[allow(dead_code)]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Load all skills from a directory.
    ///
    /// Supports multiple formats through adapters:
    /// - Single file: `name.markdown` or `name.md`
    /// - Directory: `name/SKILL.markdown`, `name/SKILL.md`, `name/CLAUDE.md`, etc.
    pub async fn load_from_directory(&self, dir: &Path) -> Result<Vec<Skill>, SkillError> {
        let mut skills = Vec::new();

        if !dir.exists() {
            debug!("Skills directory does not exist: {}", dir.display());
            return Ok(skills);
        }

        debug!("Loading skills from: {}", dir.display());

        for entry in WalkDir::new(dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                // Check for single-file skill
                if let Some(skill) = self.try_load_single_file(path) {
                    debug!("Loaded single-file skill: {}", skill.definition.id);
                    skills.push(skill);
                }
            } else if path.is_dir() && path != dir {
                // Check for directory skill
                if let Some(skill) = self.try_load_directory_skill(path) {
                    debug!("Loaded directory skill: {}", skill.definition.id);
                    skills.push(skill);
                }
            }
        }

        debug!("Loaded {} skills from {}", skills.len(), dir.display());
        Ok(skills)
    }

    /// Load a single skill from a path (file or directory).
    #[allow(dead_code)]
    pub async fn load_skill(&self, path: &Path) -> Result<Skill, SkillError> {
        if path.is_file() {
            self.load_single_file(path)
        } else if path.is_dir() {
            self.load_directory_skill(path)
        } else {
            Err(SkillError::NotFound(format!(
                "Path does not exist: {}",
                path.display()
            )))
        }
    }

    /// Try to load a single-file skill.
    fn try_load_single_file(&self, path: &Path) -> Option<Skill> {
        // Must be a .markdown or .md file
        let ext = path.extension()?.to_str()?;
        if ext != "markdown" && ext != "md" {
            return None;
        }

        // Skip SKILL.markdown files in directories (handled separately)
        let file_name = path.file_stem()?.to_str()?;
        if file_name == "SKILL" {
            return None;
        }

        match self.load_single_file(path) {
            Ok(skill) => Some(skill),
            Err(e) => {
                warn!("Failed to load skill from {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Load a single-file skill.
    fn load_single_file(&self, path: &Path) -> Result<Skill, SkillError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to read {}: {}", path.display(), e))
        })?;

        parse_skill_markdown(&content, path.parent())
    }

    /// Try to load a directory skill.
    fn try_load_directory_skill(&self, dir: &Path) -> Option<Skill> {
        // Try all supported file names
        for file_name in &self.supported_files {
            let skill_file = dir.join(file_name);
            if skill_file.exists() {
                return match self.load_skill_file(&skill_file, dir) {
                    Ok(skill) => Some(skill),
                    Err(e) => {
                        warn!(
                            "Failed to load skill from {}: {}",
                            skill_file.display(),
                            e
                        );
                        None
                    }
                };
            }
        }

        None
    }

    /// Load a directory skill.
    #[allow(dead_code)]
    fn load_directory_skill(&self, dir: &Path) -> Result<Skill, SkillError> {
        // Try all supported file names
        for file_name in &self.supported_files {
            let skill_file = dir.join(file_name);
            if skill_file.exists() {
                return self.load_skill_file(&skill_file, dir);
            }
        }

        Err(SkillError::NotFound(format!(
            "No skill file found in {} (tried: {:?})",
            dir.display(),
            self.supported_files
        )))
    }

    /// Load a skill from a specific file with a base directory.
    fn load_skill_file(&self, skill_file: &Path, base_dir: &Path) -> Result<Skill, SkillError> {
        let content = std::fs::read_to_string(skill_file).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to read {}: {}", skill_file.display(), e))
        })?;

        let file_name = skill_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("SKILL.md");

        // Use adapter registry to parse
        let registry = AdapterRegistry::new();
        registry.parse(&content, file_name, Some(base_dir))
    }
}

impl Default for FilesystemLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str, id: &str) {
        let content = format!(
            r#"---
id: {}
name: {}
description: Test skill
---

Test content for {}.
"#,
            id, name, name
        );

        fs::write(dir.join(format!("{}.markdown", id)), content).unwrap();
    }

    fn create_directory_skill(dir: &Path, id: &str) {
        let skill_dir = dir.join(id);
        fs::create_dir_all(&skill_dir).unwrap();

        let content = format!(
            r#"---
id: {}
name: {} Skill
description: A directory skill
---

Directory skill content.
"#,
            id,
            id.to_uppercase()
        );

        fs::write(skill_dir.join("SKILL.markdown"), content).unwrap();
    }

    #[tokio::test]
    async fn test_load_single_file_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_test_skill(dir, "Test One", "test-one");
        create_test_skill(dir, "Test Two", "test-two");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);

        let ids: Vec<&str> = skills.iter().map(|s| s.definition.id.as_str()).collect();
        assert!(ids.contains(&"test-one"));
        assert!(ids.contains(&"test-two"));
    }

    #[tokio::test]
    async fn test_load_directory_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "my-skill");
        create_directory_skill(dir, "another-skill");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_load_mixed_skills() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_test_skill(dir, "Single File", "single-file");
        create_directory_skill(dir, "dir-skill");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent_directory() {
        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(Path::new("/nonexistent")).await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_load_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(temp_dir.path()).await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_base_dir_in_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "with-base");

        let loader = FilesystemLoader::new();
        let skills = loader.load_from_directory(dir).await.unwrap();

        let skill = skills.iter().find(|s| s.definition.id == "with-base").unwrap();
        assert!(skill.definition.metadata.contains_key("base_dir"));
    }

    #[tokio::test]
    async fn test_load_skill_directly() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        create_directory_skill(dir, "direct-load");

        let loader = FilesystemLoader::new();
        let skill_dir = dir.join("direct-load");
        let skill = loader.load_skill(&skill_dir).await.unwrap();

        assert_eq!(skill.definition.id, "direct-load");
    }
}
