//! Bundled skill loader implementation.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition, SkillLoader};

use crate::skills::get_bundled_skills;

/// Skill loader for bundled skills.
pub struct BundledSkillLoader {
    skills: RwLock<HashMap<String, Skill>>,
}

impl BundledSkillLoader {
    /// Create a new bundled skill loader.
    pub fn new() -> Self {
        let skills: HashMap<String, Skill> = get_bundled_skills()
            .into_iter()
            .map(|s| (s.definition.id.clone(), s))
            .collect();

        Self {
            skills: RwLock::new(skills),
        }
    }
}

impl Default for BundledSkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillLoader for BundledSkillLoader {
    async fn load(&self, skill_id: &str) -> Result<Skill, SkillError> {
        let skills = self.skills.read().map_err(|_| {
            SkillError::LoadingFailed("Failed to acquire lock".to_string())
        })?;

        skills
            .get(skill_id)
            .cloned()
            .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))
    }

    async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError> {
        let skills = self.skills.read().map_err(|_| {
            SkillError::LoadingFailed("Failed to acquire lock".to_string())
        })?;

        Ok(skills.values().map(|s| s.definition.clone()).collect())
    }

    async fn reload(&self) -> Result<(), SkillError> {
        let new_skills: HashMap<String, Skill> = get_bundled_skills()
            .into_iter()
            .map(|s| (s.definition.id.clone(), s))
            .collect();

        let mut skills = self.skills.write().map_err(|_| {
            SkillError::LoadingFailed("Failed to acquire lock".to_string())
        })?;

        *skills = new_skills;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_skill() {
        let loader = BundledSkillLoader::new();
        let skill = loader.load("code-review").await.unwrap();
        assert_eq!(skill.definition.id, "code-review");
    }

    #[tokio::test]
    async fn test_load_missing_skill() {
        let loader = BundledSkillLoader::new();
        let result = loader.load("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_skills() {
        let loader = BundledSkillLoader::new();
        let skills = loader.list().await.unwrap();
        assert!(!skills.is_empty());
    }

    #[test]
    fn test_loader_new() {
        let loader = BundledSkillLoader::new();
        // Just verify it creates successfully
        let _ = loader;
    }

    #[test]
    fn test_loader_default() {
        let loader = BundledSkillLoader::default();
        let _ = loader;
    }

    #[tokio::test]
    async fn test_reload_skills() {
        let loader = BundledSkillLoader::new();
        let result = loader.reload().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_skills_not_empty() {
        let loader = BundledSkillLoader::new();
        let skills = loader.list().await.unwrap();
        // Should have at least one bundled skill
        assert!(!skills.is_empty());
    }

    #[tokio::test]
    async fn test_load_skill_error_message() {
        let loader = BundledSkillLoader::new();
        let result = loader.load("nonexistent-skill-xyz").await;
        match result {
            Err(SkillError::NotFound(id)) => {
                assert_eq!(id, "nonexistent-skill-xyz");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_reload_then_list() {
        let loader = BundledSkillLoader::new();
        loader.reload().await.unwrap();
        let skills = loader.list().await.unwrap();
        assert!(!skills.is_empty());
    }
}
