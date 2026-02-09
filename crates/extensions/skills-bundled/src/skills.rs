//! Built-in skill definitions.

use autohands_protocols::skill::{Skill, SkillDefinition, SkillVariable};

/// Get all bundled skills.
pub fn get_bundled_skills() -> Vec<Skill> {
    vec![
        code_review_skill(),
        explain_code_skill(),
        write_tests_skill(),
        refactor_skill(),
        debug_skill(),
        documentation_skill(),
    ]
}

fn code_review_skill() -> Skill {
    let def = SkillDefinition::new("code-review", "Code Review")
        .with_description("Review code for issues, improvements, and best practices");

    let def = SkillDefinition {
        category: Some("development".to_string()),
        tags: vec!["code".to_string(), "review".to_string(), "quality".to_string()],
        variables: vec![
            SkillVariable {
                name: "focus".to_string(),
                description: "Specific areas to focus on (security, performance, style)".to_string(),
                required: false,
                default: None,
            },
        ],
        required_tools: vec!["read_file".to_string(), "glob".to_string(), "grep".to_string()],
        ..def
    };

    Skill::new(def, CODE_REVIEW_CONTENT)
}

const CODE_REVIEW_CONTENT: &str = r#"You are a code reviewer. Analyze the provided code and provide feedback on:

1. **Code Quality**: Identify bugs, edge cases, and potential issues
2. **Best Practices**: Check adherence to coding standards and patterns
3. **Performance**: Look for optimization opportunities
4. **Security**: Identify potential security vulnerabilities
5. **Maintainability**: Assess readability and documentation

{{#if focus}}
Focus particularly on: {{focus}}
{{/if}}

Provide specific, actionable feedback with line references where applicable.
"#;

fn explain_code_skill() -> Skill {
    let def = SkillDefinition::new("explain-code", "Explain Code")
        .with_description("Explain how code works in plain language");

    let def = SkillDefinition {
        category: Some("learning".to_string()),
        tags: vec!["code".to_string(), "explain".to_string(), "learning".to_string()],
        variables: vec![
            SkillVariable {
                name: "audience".to_string(),
                description: "Target audience (beginner, intermediate, expert)".to_string(),
                required: false,
                default: Some("intermediate".to_string()),
            },
        ],
        required_tools: vec!["read_file".to_string()],
        ..def
    };

    Skill::new(def, EXPLAIN_CODE_CONTENT)
}

const EXPLAIN_CODE_CONTENT: &str = r#"Explain the provided code clearly and thoroughly.

Target audience: {{audience}}

Structure your explanation:
1. **Overview**: What does this code do at a high level?
2. **Key Components**: Break down the main parts
3. **Flow**: Explain how data/control flows through the code
4. **Important Details**: Highlight clever techniques or gotchas

Use analogies and examples appropriate for the audience level.
"#;

fn write_tests_skill() -> Skill {
    let def = SkillDefinition::new("write-tests", "Write Tests")
        .with_description("Generate comprehensive test cases for code");

    let def = SkillDefinition {
        category: Some("development".to_string()),
        tags: vec!["testing".to_string(), "quality".to_string()],
        variables: vec![
            SkillVariable {
                name: "framework".to_string(),
                description: "Test framework to use".to_string(),
                required: false,
                default: None,
            },
        ],
        required_tools: vec!["read_file".to_string(), "write_file".to_string()],
        ..def
    };

    Skill::new(def, WRITE_TESTS_CONTENT)
}

const WRITE_TESTS_CONTENT: &str = r#"Generate comprehensive tests for the provided code.

{{#if framework}}
Use the {{framework}} testing framework.
{{/if}}

Include tests for:
1. **Happy Path**: Normal expected inputs
2. **Edge Cases**: Boundary conditions, empty inputs, etc.
3. **Error Cases**: Invalid inputs, error handling
4. **Integration**: How components work together (if applicable)

Follow testing best practices:
- Clear test names that describe the scenario
- One assertion per test where practical
- Proper setup and teardown
- Mock external dependencies
"#;

fn refactor_skill() -> Skill {
    let def = SkillDefinition::new("refactor", "Refactor Code")
        .with_description("Improve code structure without changing behavior");

    let def = SkillDefinition {
        category: Some("development".to_string()),
        tags: vec!["refactoring".to_string(), "clean-code".to_string()],
        variables: vec![
            SkillVariable {
                name: "goal".to_string(),
                description: "Specific refactoring goal (e.g., extract method, reduce duplication)".to_string(),
                required: false,
                default: None,
            },
        ],
        required_tools: vec!["read_file".to_string(), "edit_file".to_string()],
        ..def
    };

    Skill::new(def, REFACTOR_CONTENT)
}

const REFACTOR_CONTENT: &str = r#"Refactor the provided code to improve its quality.

{{#if goal}}
Focus on: {{goal}}
{{/if}}

Apply these refactoring principles:
- **DRY**: Eliminate duplication
- **SRP**: Each function/class should have one responsibility
- **KISS**: Keep it simple
- **Extract**: Create well-named helper functions
- **Rename**: Use clear, descriptive names

Preserve behavior exactly - all existing tests should pass.
Explain each change and why it improves the code.
"#;

fn debug_skill() -> Skill {
    let def = SkillDefinition::new("debug", "Debug Issue")
        .with_description("Help diagnose and fix bugs in code");

    let def = SkillDefinition {
        category: Some("development".to_string()),
        tags: vec!["debugging".to_string(), "troubleshooting".to_string()],
        variables: vec![
            SkillVariable {
                name: "error".to_string(),
                description: "Error message or description of the problem".to_string(),
                required: true,
                default: None,
            },
        ],
        required_tools: vec!["read_file".to_string(), "grep".to_string(), "exec".to_string()],
        ..def
    };

    Skill::new(def, DEBUG_CONTENT)
}

const DEBUG_CONTENT: &str = r#"Help debug the following issue:

Error/Problem: {{error}}

Debugging approach:
1. **Reproduce**: Understand when and how the issue occurs
2. **Isolate**: Narrow down to the specific cause
3. **Analyze**: Examine the code and data flow
4. **Hypothesize**: Form theories about the root cause
5. **Test**: Verify the hypothesis
6. **Fix**: Implement and validate the solution

Use available tools to examine the code, run tests, and verify the fix.
"#;

fn documentation_skill() -> Skill {
    let def = SkillDefinition::new("documentation", "Write Documentation")
        .with_description("Generate documentation for code");

    let def = SkillDefinition {
        category: Some("documentation".to_string()),
        tags: vec!["docs".to_string(), "documentation".to_string()],
        variables: vec![
            SkillVariable {
                name: "style".to_string(),
                description: "Documentation style (jsdoc, rustdoc, docstring, markdown)".to_string(),
                required: false,
                default: None,
            },
        ],
        required_tools: vec!["read_file".to_string(), "edit_file".to_string()],
        ..def
    };

    Skill::new(def, DOCUMENTATION_CONTENT)
}

const DOCUMENTATION_CONTENT: &str = r#"Generate documentation for the provided code.

{{#if style}}
Use {{style}} documentation style.
{{/if}}

Include:
1. **Overview**: What the code does
2. **Parameters**: Document all function/method parameters
3. **Returns**: What is returned and when
4. **Examples**: Usage examples
5. **Errors**: What errors can occur and why

Write clear, concise documentation that helps users understand and use the code correctly.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_skills_not_empty() {
        let skills = get_bundled_skills();
        assert!(!skills.is_empty());
    }

    #[test]
    fn test_skill_render() {
        let skills = get_bundled_skills();
        let review = skills.iter().find(|s| s.definition.id == "code-review").unwrap();

        let mut vars = std::collections::HashMap::new();
        vars.insert("focus".to_string(), "security".to_string());

        let rendered = review.render(&vars);
        assert!(rendered.contains("security"));
    }
}
