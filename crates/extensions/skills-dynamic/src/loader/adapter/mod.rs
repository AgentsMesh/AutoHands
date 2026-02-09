//! Skill format adapters.
//!
//! Provides adapters for converting different skill formats to AutoHands native format.
//! Each adapter handles a specific external format (Claude Code, OpenClaw, Microsoft, etc.)

mod autohands;
mod claude_code;
mod microsoft;
mod openclaw;

pub use autohands::AutoHandsAdapter;
pub use claude_code::ClaudeCodeAdapter;
pub use microsoft::MicrosoftAdapter;
pub use openclaw::OpenClawAdapter;

use std::path::Path;

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::Skill;

/// Trait for skill format adapters.
///
/// Each adapter knows how to:
/// 1. Detect if a skill file matches its format
/// 2. Parse the file and convert to AutoHands native format
pub trait SkillAdapter: Send + Sync {
    /// Adapter name for logging/debugging.
    fn name(&self) -> &'static str;

    /// Check if this adapter can handle the given skill file.
    ///
    /// # Arguments
    /// * `content` - The raw content of the skill file
    /// * `file_name` - The name of the skill file (e.g., "SKILL.md", "CLAUDE.md")
    fn can_handle(&self, content: &str, file_name: &str) -> bool;

    /// Parse the skill file and convert to AutoHands format.
    ///
    /// # Arguments
    /// * `content` - The raw content of the skill file
    /// * `base_dir` - The directory containing the skill
    fn parse(&self, content: &str, base_dir: Option<&Path>) -> Result<Skill, SkillError>;

    /// Get the list of file names this adapter looks for.
    fn supported_file_names(&self) -> &[&'static str];
}

/// Adapter registry for managing multiple format adapters.
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SkillAdapter>>,
}

impl AdapterRegistry {
    /// Create a new registry with all built-in adapters.
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(AutoHandsAdapter::new()),
                Box::new(ClaudeCodeAdapter::new()),
                Box::new(OpenClawAdapter::new()),
                Box::new(MicrosoftAdapter::new()),
            ],
        }
    }

    /// Add a custom adapter to the registry.
    #[allow(dead_code)]
    pub fn register(&mut self, adapter: Box<dyn SkillAdapter>) {
        self.adapters.push(adapter);
    }

    /// Get all supported file names across all adapters.
    pub fn supported_file_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self
            .adapters
            .iter()
            .flat_map(|a| a.supported_file_names().iter().copied())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Find an adapter that can handle the given content and file name.
    pub fn find_adapter(&self, content: &str, file_name: &str) -> Option<&dyn SkillAdapter> {
        self.adapters
            .iter()
            .find(|a| a.can_handle(content, file_name))
            .map(|a| a.as_ref())
    }

    /// Parse a skill file using the appropriate adapter.
    pub fn parse(
        &self,
        content: &str,
        file_name: &str,
        base_dir: Option<&Path>,
    ) -> Result<Skill, SkillError> {
        let adapter = self.find_adapter(content, file_name).ok_or_else(|| {
            SkillError::ParsingError(format!(
                "No adapter found for skill file: {}",
                file_name
            ))
        })?;

        tracing::debug!("Using {} adapter for {}", adapter.name(), file_name);
        adapter.parse(content, base_dir)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = AdapterRegistry::new();
        assert!(!registry.adapters.is_empty());
    }

    #[test]
    fn test_supported_file_names() {
        let registry = AdapterRegistry::new();
        let names = registry.supported_file_names();
        assert!(names.contains(&"SKILL.markdown"));
        assert!(names.contains(&"SKILL.md"));
        assert!(names.contains(&"CLAUDE.md"));
    }
}
