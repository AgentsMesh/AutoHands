//! Progressive disclosure for skills.
//!
//! Implements Claude Code-style 3-level progressive disclosure:
//! - L1: Skill metadata (name + description) always in System Prompt
//! - L2: Full skill content loaded on-demand via `skill_content` tool
//! - L3: Skill resources loaded on-demand via `skill_read` tool

use crate::registry::SkillRegistry;
use std::sync::Arc;

/// Generates skill metadata section for injection into System Prompt.
///
/// This implements Level 1 (L1) of progressive disclosure - the model
/// always sees a summary of available skills, enabling it to decide
/// when to load full skill content.
pub struct SkillMetadataInjector {
    registry: Arc<SkillRegistry>,
}

impl SkillMetadataInjector {
    /// Create a new metadata injector.
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }

    /// Generate the `<available_skills>` section for System Prompt.
    ///
    /// Format follows Claude Code's approach:
    /// ```xml
    /// <available_skills>
    ///   <skill>
    ///     <id>code-review</id>
    ///     <name>Code Review Expert</name>
    ///     <description>Expert code reviewer...</description>
    ///     <tags>development, review</tags>
    ///   </skill>
    ///   ...
    /// </available_skills>
    /// ```
    pub async fn generate_metadata_section(&self) -> String {
        let skills = self.registry.list().await;

        if skills.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("<available_skills>\n");

        for skill in &skills {
            output.push_str("  <skill>\n");
            output.push_str(&format!("    <id>{}</id>\n", xml_escape(&skill.id)));
            output.push_str(&format!("    <name>{}</name>\n", xml_escape(&skill.name)));
            output.push_str(&format!(
                "    <description>{}</description>\n",
                xml_escape(&skill.description)
            ));

            if !skill.tags.is_empty() {
                output.push_str(&format!(
                    "    <tags>{}</tags>\n",
                    skill.tags.join(", ")
                ));
            }

            if let Some(ref category) = skill.category {
                output.push_str(&format!("    <category>{}</category>\n", xml_escape(category)));
            }

            output.push_str("  </skill>\n");
        }

        output.push_str("</available_skills>");
        output
    }

    /// Generate the instruction section for how to use skills.
    ///
    /// This tells the model how to activate skills when appropriate.
    pub fn generate_instruction_section(&self) -> String {
        r#"
## Skills System

You have access to a collection of skills that provide expert guidance for specific tasks.
The available skills are listed in the <available_skills> section above.

### How to Use Skills

1. **Skill Discovery**: Review the available skills and their descriptions
2. **Skill Activation**: When a task matches a skill's purpose, use the `skill_content` tool to load the full guidance
3. **Follow the Guidance**: Once loaded, follow the skill's instructions to complete the task
4. **Access Resources**: Some skills have additional files - use `skill_read` to access them if needed

### When to Activate a Skill

- If exactly ONE skill clearly matches the current task, load it immediately
- If MULTIPLE skills might apply, consider the most specific one for the task
- If NO skill applies, proceed with your general capabilities

### Skill Tools Available

- `skill_list`: List all available skills (with optional tag/category filter)
- `skill_info`: Get detailed information about a specific skill
- `skill_content`: Load the full expert guidance from a skill
- `skill_read`: Read additional resource files from a skill's directory
"#.to_string()
    }

    /// Generate complete System Prompt section for skills.
    ///
    /// Combines metadata listing and usage instructions.
    pub async fn generate_system_prompt_section(&self) -> String {
        let metadata = self.generate_metadata_section().await;
        if metadata.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("\n\n");
        output.push_str(&metadata);
        output.push_str("\n");
        output.push_str(&self.generate_instruction_section());
        output
    }
}

/// Simple XML escaping for safety.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::skill::{Skill, SkillDefinition};

    async fn create_test_registry() -> Arc<SkillRegistry> {
        let registry = Arc::new(SkillRegistry::new());

        let mut def1 = SkillDefinition::new("code-review", "Code Review Expert");
        def1.description = "Expert code reviewer for identifying issues".to_string();
        def1.tags = vec!["development".to_string(), "review".to_string()];
        def1.category = Some("development".to_string());
        registry.register(Skill::new(def1, "Full content here")).await;

        let mut def2 = SkillDefinition::new("security-audit", "Security Audit");
        def2.description = "Security vulnerability scanner".to_string();
        def2.tags = vec!["security".to_string()];
        registry.register(Skill::new(def2, "Security content")).await;

        registry
    }

    #[tokio::test]
    async fn test_generate_metadata_section() {
        let registry = create_test_registry().await;
        let injector = SkillMetadataInjector::new(registry);

        let section = injector.generate_metadata_section().await;

        assert!(section.contains("<available_skills>"));
        assert!(section.contains("</available_skills>"));
        assert!(section.contains("<id>code-review</id>"));
        assert!(section.contains("<name>Code Review Expert</name>"));
        assert!(section.contains("<tags>development, review</tags>"));
        assert!(section.contains("<category>development</category>"));
        assert!(section.contains("<id>security-audit</id>"));
    }

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = Arc::new(SkillRegistry::new());
        let injector = SkillMetadataInjector::new(registry);

        let section = injector.generate_metadata_section().await;
        assert!(section.is_empty());
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("<test>"), "&lt;test&gt;");
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("\"quote\""), "&quot;quote&quot;");
    }

    #[tokio::test]
    async fn test_generate_system_prompt_section() {
        let registry = create_test_registry().await;
        let injector = SkillMetadataInjector::new(registry);

        let section = injector.generate_system_prompt_section().await;

        // Should contain both metadata and instructions
        assert!(section.contains("<available_skills>"));
        assert!(section.contains("## Skills System"));
        assert!(section.contains("skill_content"));
        assert!(section.contains("skill_read"));
    }

    #[tokio::test]
    async fn test_instruction_section() {
        let registry = Arc::new(SkillRegistry::new());
        let injector = SkillMetadataInjector::new(registry);

        let instructions = injector.generate_instruction_section();

        assert!(instructions.contains("Skill Discovery"));
        assert!(instructions.contains("Skill Activation"));
        assert!(instructions.contains("skill_list"));
        assert!(instructions.contains("skill_info"));
    }
}
