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
#[path = "context_builder_tests.rs"]
mod tests;
