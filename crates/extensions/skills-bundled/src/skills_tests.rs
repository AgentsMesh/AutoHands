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
