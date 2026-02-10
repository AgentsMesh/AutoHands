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
