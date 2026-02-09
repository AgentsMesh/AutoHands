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
mod tests {
    use super::*;
    use autohands_protocols::skill::SkillDefinition;

    fn create_test_skill(id: &str, tags: Vec<&str>, category: Option<&str>) -> Skill {
        let mut def = SkillDefinition::new(id, &format!("{} Skill", id));
        def.tags = tags.into_iter().map(String::from).collect();
        def.category = category.map(String::from);
        Skill::new(def, "Test content")
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-1", vec!["a", "b"], Some("cat1"));

        registry.register(skill).await;

        let retrieved = registry.get("test-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().definition.id, "test-1");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("test-1", vec!["a"], None)).await;

        let removed = registry.unregister("test-1").await;
        assert!(removed.is_some());

        let get_result = registry.get("test-1").await;
        assert!(get_result.is_none());
    }

    #[tokio::test]
    async fn test_list() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("skill-1", vec![], None)).await;
        registry.register(create_test_skill("skill-2", vec![], None)).await;

        let list = registry.list().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_tag() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("s1", vec!["rust", "async"], None)).await;
        registry.register(create_test_skill("s2", vec!["rust", "web"], None)).await;
        registry.register(create_test_skill("s3", vec!["python"], None)).await;

        let rust_skills = registry.find_by_tag("rust").await;
        assert_eq!(rust_skills.len(), 2);

        let async_skills = registry.find_by_tag("async").await;
        assert_eq!(async_skills.len(), 1);

        let nonexistent = registry.find_by_tag("nonexistent").await;
        assert!(nonexistent.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_category() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("s1", vec![], Some("development"))).await;
        registry.register(create_test_skill("s2", vec![], Some("development"))).await;
        registry.register(create_test_skill("s3", vec![], Some("testing"))).await;

        let dev_skills = registry.find_by_category("development").await;
        assert_eq!(dev_skills.len(), 2);
    }

    #[tokio::test]
    async fn test_tags_and_categories() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("s1", vec!["a", "b"], Some("cat1"))).await;
        registry.register(create_test_skill("s2", vec!["b", "c"], Some("cat2"))).await;

        let tags = registry.tags().await;
        assert_eq!(tags.len(), 3);

        let cats = registry.categories().await;
        assert_eq!(cats.len(), 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("s1", vec!["a"], Some("cat1"))).await;
        registry.register(create_test_skill("s2", vec!["b"], Some("cat2"))).await;

        registry.clear().await;

        assert!(registry.is_empty().await);
        assert!(registry.tags().await.is_empty());
        assert!(registry.categories().await.is_empty());
    }

    #[tokio::test]
    async fn test_replace_all() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("old-1", vec![], None)).await;
        registry.register(create_test_skill("old-2", vec![], None)).await;

        let new_skills = vec![
            create_test_skill("new-1", vec!["x"], None),
            create_test_skill("new-2", vec!["y"], None),
            create_test_skill("new-3", vec!["z"], None),
        ];

        registry.replace_all(new_skills).await;

        assert_eq!(registry.len().await, 3);
        assert!(registry.get("old-1").await.is_none());
        assert!(registry.get("new-1").await.is_some());
    }

    #[tokio::test]
    async fn test_contains() {
        let registry = SkillRegistry::new();
        registry.register(create_test_skill("exists", vec![], None)).await;

        assert!(registry.contains("exists").await);
        assert!(!registry.contains("not-exists").await);
    }
}
