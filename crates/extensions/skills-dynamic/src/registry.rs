//! Skill registry for managing loaded skills.
//!
//! Provides a thread-safe registry for accessing skills by ID or tag.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::debug;

use autohands_protocols::skill::{Skill, SkillDefinition};

/// Thread-safe skill registry.
pub struct SkillRegistry {
    /// Skills indexed by ID.
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    /// Skills indexed by tag.
    tags_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Skills indexed by category.
    category_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SkillRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            tags_index: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a skill.
    pub async fn register(&self, skill: Skill) {
        let id = skill.definition.id.clone();

        // Update indexes
        let tags = skill.definition.tags.clone();
        let category = skill.definition.category.clone();

        // Insert skill
        {
            let mut skills = self.skills.write().await;
            skills.insert(id.clone(), skill);
        }

        // Update tag index
        {
            let mut tags_index = self.tags_index.write().await;
            for tag in tags {
                tags_index
                    .entry(tag)
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }
        }

        // Update category index
        if let Some(cat) = category {
            let mut cat_index = self.category_index.write().await;
            cat_index.entry(cat).or_insert_with(Vec::new).push(id.clone());
        }

        debug!("Registered skill: {}", id);
    }

    /// Unregister a skill.
    pub async fn unregister(&self, skill_id: &str) -> Option<Skill> {
        // Remove from skills
        let skill = {
            let mut skills = self.skills.write().await;
            skills.remove(skill_id)
        };

        if let Some(ref s) = skill {
            // Remove from tag index
            {
                let mut tags_index = self.tags_index.write().await;
                for tag in &s.definition.tags {
                    if let Some(ids) = tags_index.get_mut(tag) {
                        ids.retain(|id| id != skill_id);
                    }
                }
            }

            // Remove from category index
            if let Some(ref cat) = s.definition.category {
                let mut cat_index = self.category_index.write().await;
                if let Some(ids) = cat_index.get_mut(cat) {
                    ids.retain(|id| id != skill_id);
                }
            }

            debug!("Unregistered skill: {}", skill_id);
        }

        skill
    }

    /// Get a skill by ID.
    pub async fn get(&self, skill_id: &str) -> Option<Skill> {
        let skills = self.skills.read().await;
        skills.get(skill_id).cloned()
    }

    /// Check if a skill exists.
    pub async fn contains(&self, skill_id: &str) -> bool {
        let skills = self.skills.read().await;
        skills.contains_key(skill_id)
    }

    /// List all skill definitions.
    pub async fn list(&self) -> Vec<SkillDefinition> {
        let skills = self.skills.read().await;
        skills.values().map(|s| s.definition.clone()).collect()
    }

    /// List all skill IDs.
    pub async fn list_ids(&self) -> Vec<String> {
        let skills = self.skills.read().await;
        skills.keys().cloned().collect()
    }

    /// Find skills by tag.
    pub async fn find_by_tag(&self, tag: &str) -> Vec<Skill> {
        let tags_index = self.tags_index.read().await;
        let skills = self.skills.read().await;

        tags_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| skills.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find skills by category.
    pub async fn find_by_category(&self, category: &str) -> Vec<Skill> {
        let cat_index = self.category_index.read().await;
        let skills = self.skills.read().await;

        cat_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| skills.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all unique tags.
    pub async fn tags(&self) -> Vec<String> {
        let tags_index = self.tags_index.read().await;
        tags_index.keys().cloned().collect()
    }

    /// Get all unique categories.
    pub async fn categories(&self) -> Vec<String> {
        let cat_index = self.category_index.read().await;
        cat_index.keys().cloned().collect()
    }

    /// Get the number of registered skills.
    pub async fn len(&self) -> usize {
        let skills = self.skills.read().await;
        skills.len()
    }

    /// Check if the registry is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Clear all skills.
    pub async fn clear(&self) {
        {
            let mut skills = self.skills.write().await;
            skills.clear();
        }
        {
            let mut tags_index = self.tags_index.write().await;
            tags_index.clear();
        }
        {
            let mut cat_index = self.category_index.write().await;
            cat_index.clear();
        }
    }

    /// Replace all skills (for bulk reload).
    pub async fn replace_all(&self, new_skills: Vec<Skill>) {
        self.clear().await;

        for skill in new_skills {
            self.register(skill).await;
        }
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
