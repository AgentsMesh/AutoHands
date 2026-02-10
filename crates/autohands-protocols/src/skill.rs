//! Skill protocol definitions.
//!
//! Skills are reusable prompt templates that provide specialized capabilities.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::SkillError;
use crate::types::Metadata;

/// Core trait for skill loaders.
#[async_trait]
pub trait SkillLoader: Send + Sync {
    /// Load a skill by ID.
    async fn load(&self, skill_id: &str) -> Result<Skill, SkillError>;

    /// List all available skills.
    async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError>;

    /// Reload skills from source.
    async fn reload(&self) -> Result<(), SkillError>;
}

/// A skill instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill definition.
    pub definition: SkillDefinition,

    /// The skill content (prompt template).
    pub content: String,

    /// Parsed sections of the skill.
    #[serde(default)]
    pub sections: HashMap<String, String>,
}

impl Skill {
    pub fn new(definition: SkillDefinition, content: impl Into<String>) -> Self {
        Self {
            definition,
            content: content.into(),
            sections: HashMap::new(),
        }
    }

    /// Render the skill with given variables.
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.content.clone();
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Definition/metadata for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Unique identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of what the skill does.
    pub description: String,

    /// Category for organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Tags for discovery.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Required variables.
    #[serde(default)]
    pub variables: Vec<SkillVariable>,

    /// Tool IDs this skill requires.
    #[serde(default)]
    pub required_tools: Vec<String>,

    /// Whether this skill is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Priority for skill selection (higher = more preferred).
    #[serde(default)]
    pub priority: i32,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

fn default_true() -> bool {
    true
}

impl SkillDefinition {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category: None,
            tags: Vec::new(),
            variables: Vec::new(),
            required_tools: Vec::new(),
            enabled: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// A variable required by a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVariable {
    /// Variable name.
    pub name: String,

    /// Description of the variable.
    pub description: String,

    /// Whether this variable is required.
    #[serde(default = "default_true")]
    pub required: bool,

    /// Default value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[cfg(test)]
#[path = "skill_tests.rs"]
mod tests;
