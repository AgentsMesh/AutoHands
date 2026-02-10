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
