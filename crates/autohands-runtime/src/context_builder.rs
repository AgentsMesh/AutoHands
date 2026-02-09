//! Context builder for constructing agent execution context.

use std::collections::HashMap;
use std::sync::Arc;

use autohands_core::registry::ToolRegistry;
use autohands_protocols::provider::CompletionRequest;
use autohands_protocols::skill::Skill;
use autohands_protocols::tool::ToolDefinition;
use autohands_protocols::types::Message;

/// Builder for constructing completion request context.
pub struct ContextBuilder {
    system_prompt: Option<String>,
    skills: Vec<Skill>,
    skill_variables: HashMap<String, String>,
    tool_definitions: Vec<ToolDefinition>,
    messages: Vec<Message>,
    model: String,
}

impl ContextBuilder {
    /// Create a new context builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            system_prompt: None,
            skills: Vec::new(),
            skill_variables: HashMap::new(),
            tool_definitions: Vec::new(),
            messages: Vec::new(),
            model: model.into(),
        }
    }

    /// Set the base system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a skill to inject into the context.
    pub fn with_skill(mut self, skill: Skill) -> Self {
        self.skills.push(skill);
        self
    }

    /// Add multiple skills.
    pub fn with_skills(mut self, skills: Vec<Skill>) -> Self {
        self.skills.extend(skills);
        self
    }

    /// Set variables for skill rendering.
    pub fn with_skill_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.skill_variables = variables;
        self
    }

    /// Add a tool definition.
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.tool_definitions.push(tool);
        self
    }

    /// Add tools from a registry.
    pub fn with_tools_from_registry(mut self, registry: &Arc<ToolRegistry>) -> Self {
        self.tool_definitions.extend(registry.list());
        self
    }

    /// Add tools by IDs from a registry.
    pub fn with_tools_by_ids(
        mut self,
        registry: &Arc<ToolRegistry>,
        tool_ids: &[String],
    ) -> Self {
        for id in tool_ids {
            if let Some(tool) = registry.get(id) {
                self.tool_definitions.push(tool.definition().clone());
            }
        }
        self
    }

    /// Set the conversation messages.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Build the final system prompt.
    fn build_system_prompt(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Base system prompt
        if let Some(ref prompt) = self.system_prompt {
            parts.push(prompt.clone());
        }

        // Inject skills
        for skill in &self.skills {
            let rendered = skill.render(&self.skill_variables);
            parts.push(format!(
                "## Skill: {}\n\n{}",
                skill.definition.name, rendered
            ));
        }

        // Tool descriptions (optional, LLM can also use function calling)
        if !self.tool_definitions.is_empty() {
            let tool_section = self.build_tools_section();
            if !tool_section.is_empty() {
                parts.push(tool_section);
            }
        }

        parts.join("\n\n")
    }

    /// Build tool descriptions section.
    fn build_tools_section(&self) -> String {
        // This is optional since tools are usually injected via function calling
        // But can be useful for models that prefer inline tool descriptions
        String::new()
    }

    /// Build the completion request.
    pub fn build(self) -> CompletionRequest {
        let system_prompt = self.build_system_prompt();

        CompletionRequest::new(self.model, self.messages)
            .with_system(system_prompt)
            .with_tools(self.tool_definitions)
    }
}

/// Configuration for context building.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Whether to include tool descriptions in system prompt.
    pub inline_tool_descriptions: bool,

    /// Maximum tokens for the context.
    pub max_context_tokens: u32,

    /// Whether to truncate history if context is too long.
    pub auto_truncate: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            inline_tool_descriptions: false,
            max_context_tokens: 100_000,
            auto_truncate: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::skill::SkillDefinition;

    #[test]
    fn test_context_builder_new() {
        let builder = ContextBuilder::new("gpt-4");
        let request = builder.build();
        assert_eq!(request.model, "gpt-4");
    }

    #[test]
    fn test_with_system_prompt() {
        let builder = ContextBuilder::new("gpt-4").with_system_prompt("You are a helpful assistant.");
        let request = builder.build();
        assert!(request.system.is_some());
        assert!(request.system.unwrap().contains("helpful assistant"));
    }

    #[test]
    fn test_with_skill() {
        let skill = Skill::new(
            SkillDefinition::new("test", "Test Skill"),
            "This is a test skill content.",
        );

        let builder = ContextBuilder::new("gpt-4")
            .with_system_prompt("Base prompt.")
            .with_skill(skill);

        let request = builder.build();
        let system = request.system.unwrap();
        assert!(system.contains("Base prompt"));
        assert!(system.contains("Test Skill"));
        assert!(system.contains("test skill content"));
    }

    #[test]
    fn test_with_skill_variables() {
        let skill = Skill::new(
            SkillDefinition::new("greeting", "Greeting"),
            "Hello, {{name}}!",
        );

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "World".to_string());

        let builder = ContextBuilder::new("gpt-4")
            .with_skill(skill)
            .with_skill_variables(variables);

        let request = builder.build();
        let system = request.system.unwrap();
        assert!(system.contains("Hello, World!"));
    }

    #[test]
    fn test_with_tool() {
        let tool = ToolDefinition::new("read_file", "Read File", "Read file content");

        let builder = ContextBuilder::new("gpt-4").with_tool(tool);
        let request = builder.build();

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].id, "read_file");
    }

    #[test]
    fn test_with_messages() {
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let builder = ContextBuilder::new("gpt-4").with_messages(messages);
        let request = builder.build();

        assert_eq!(request.messages.len(), 2);
    }

    #[test]
    fn test_context_config_default() {
        let config = ContextConfig::default();
        assert!(!config.inline_tool_descriptions);
        assert_eq!(config.max_context_tokens, 100_000);
        assert!(config.auto_truncate);
    }

    #[test]
    fn test_multiple_skills() {
        let skill1 = Skill::new(SkillDefinition::new("s1", "Skill 1"), "Content 1");
        let skill2 = Skill::new(SkillDefinition::new("s2", "Skill 2"), "Content 2");

        let builder = ContextBuilder::new("gpt-4").with_skills(vec![skill1, skill2]);

        let request = builder.build();
        let system = request.system.unwrap();
        assert!(system.contains("Skill 1"));
        assert!(system.contains("Skill 2"));
        assert!(system.contains("Content 1"));
        assert!(system.contains("Content 2"));
    }

    #[test]
    fn test_tools_from_registry() {
        let registry = Arc::new(ToolRegistry::new());
        // Registry is empty, should still work
        let builder = ContextBuilder::new("gpt-4").with_tools_from_registry(&registry);
        let request = builder.build();
        assert!(request.tools.is_empty());
    }

    #[test]
    fn test_full_builder_chain() {
        let skill = Skill::new(
            SkillDefinition::new("coding", "Coding Skill"),
            "You are an expert programmer.",
        );
        let tool = ToolDefinition::new("write_file", "Write File", "Write to a file");
        let messages = vec![Message::user("Write a hello world program")];

        let builder = ContextBuilder::new("claude-3-5-sonnet-20241022")
            .with_system_prompt("You are a helpful coding assistant.")
            .with_skill(skill)
            .with_tool(tool)
            .with_messages(messages);

        let request = builder.build();

        assert_eq!(request.model, "claude-3-5-sonnet-20241022");
        assert!(request.system.is_some());
        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.messages.len(), 1);

        let system = request.system.unwrap();
        assert!(system.contains("helpful coding assistant"));
        assert!(system.contains("Coding Skill"));
        assert!(system.contains("expert programmer"));
    }

    #[test]
    fn test_context_config_debug() {
        let config = ContextConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("ContextConfig"));
    }

    #[test]
    fn test_context_config_clone() {
        let config = ContextConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_context_tokens, config.max_context_tokens);
    }

    #[test]
    fn test_context_config_custom() {
        let config = ContextConfig {
            inline_tool_descriptions: true,
            max_context_tokens: 50_000,
            auto_truncate: false,
        };
        assert!(config.inline_tool_descriptions);
        assert_eq!(config.max_context_tokens, 50_000);
        assert!(!config.auto_truncate);
    }

    #[test]
    fn test_builder_without_system_prompt() {
        let builder = ContextBuilder::new("gpt-4");
        let request = builder.build();
        // Should have empty system prompt
        assert_eq!(request.system, Some(String::new()));
    }

    #[test]
    fn test_builder_with_empty_tools() {
        let builder = ContextBuilder::new("gpt-4")
            .with_system_prompt("Test prompt");
        let request = builder.build();
        assert!(request.tools.is_empty());
    }

    #[test]
    fn test_tools_by_ids_with_empty_registry() {
        let registry = Arc::new(ToolRegistry::new());
        let tool_ids = vec!["nonexistent".to_string()];
        let builder = ContextBuilder::new("gpt-4")
            .with_tools_by_ids(&registry, &tool_ids);
        let request = builder.build();
        // No tools should be added since registry is empty
        assert!(request.tools.is_empty());
    }

    #[test]
    fn test_multiple_tools() {
        let tool1 = ToolDefinition::new("tool1", "Tool 1", "Description 1");
        let tool2 = ToolDefinition::new("tool2", "Tool 2", "Description 2");

        let builder = ContextBuilder::new("gpt-4")
            .with_tool(tool1)
            .with_tool(tool2);
        let request = builder.build();

        assert_eq!(request.tools.len(), 2);
    }

    #[test]
    fn test_skill_without_variables() {
        let skill = Skill::new(
            SkillDefinition::new("test", "Test Skill"),
            "Static content with no variables.",
        );

        let builder = ContextBuilder::new("gpt-4").with_skill(skill);
        let request = builder.build();
        let system = request.system.unwrap();
        assert!(system.contains("Static content with no variables"));
    }

    #[test]
    fn test_builder_model_string() {
        let builder = ContextBuilder::new(String::from("claude-opus-4"));
        let request = builder.build();
        assert_eq!(request.model, "claude-opus-4");
    }
}
