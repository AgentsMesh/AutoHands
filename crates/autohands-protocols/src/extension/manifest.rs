//! Extension manifest types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Author, Metadata, Permission, Version};

/// Extension manifest containing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    #[serde(default)]
    pub dependencies: Dependencies,
    #[serde(default)]
    pub provides: Provides,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl ExtensionManifest {
    /// Create a new extension manifest.
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: Version) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version,
            description: String::new(),
            author: None,
            dependencies: Dependencies::default(),
            provides: Provides::default(),
            config_schema: None,
            permissions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_author(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }
}

/// Dependencies on other extensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub required: Vec<DependencySpec>,
    #[serde(default)]
    pub optional: Vec<DependencySpec>,
}

/// Specification for a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// What an extension provides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provides {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub memory_backends: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
