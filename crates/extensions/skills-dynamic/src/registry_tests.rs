use super::*;
use autohands_protocols::skill::SkillDefinition;

fn create_test_skill(id: &str, tags: Vec<&str>, category: Option<&str>) -> Skill {
    let mut def = SkillDefinition::new(id, &format!("{} Skill", id));
    def.tags = tags.into_iter().map(String::from).collect();
    def.category = category.map(String::from);
    Skill::new(def, "Test content")
}

#[tokio::test]
async fn test_register_and_get() {
    let registry = SkillRegistry::new();
    let skill = create_test_skill("test-1", vec!["a", "b"], Some("cat1"));

    registry.register(skill).await;

    let retrieved = registry.get("test-1").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().definition.id, "test-1");
}

#[tokio::test]
async fn test_unregister() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("test-1", vec!["a"], None)).await;

    let removed = registry.unregister("test-1").await;
    assert!(removed.is_some());

    let get_result = registry.get("test-1").await;
    assert!(get_result.is_none());
}

#[tokio::test]
async fn test_list() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("skill-1", vec![], None)).await;
    registry.register(create_test_skill("skill-2", vec![], None)).await;

    let list = registry.list().await;
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_find_by_tag() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("s1", vec!["rust", "async"], None)).await;
    registry.register(create_test_skill("s2", vec!["rust", "web"], None)).await;
    registry.register(create_test_skill("s3", vec!["python"], None)).await;

    let rust_skills = registry.find_by_tag("rust").await;
    assert_eq!(rust_skills.len(), 2);

    let async_skills = registry.find_by_tag("async").await;
    assert_eq!(async_skills.len(), 1);

    let nonexistent = registry.find_by_tag("nonexistent").await;
    assert!(nonexistent.is_empty());
}

#[tokio::test]
async fn test_find_by_category() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("s1", vec![], Some("development"))).await;
    registry.register(create_test_skill("s2", vec![], Some("development"))).await;
    registry.register(create_test_skill("s3", vec![], Some("testing"))).await;

    let dev_skills = registry.find_by_category("development").await;
    assert_eq!(dev_skills.len(), 2);
}

#[tokio::test]
async fn test_tags_and_categories() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("s1", vec!["a", "b"], Some("cat1"))).await;
    registry.register(create_test_skill("s2", vec!["b", "c"], Some("cat2"))).await;

    let tags = registry.tags().await;
    assert_eq!(tags.len(), 3);

    let cats = registry.categories().await;
    assert_eq!(cats.len(), 2);
}

#[tokio::test]
async fn test_clear() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("s1", vec!["a"], Some("cat1"))).await;
    registry.register(create_test_skill("s2", vec!["b"], Some("cat2"))).await;

    registry.clear().await;

    assert!(registry.is_empty().await);
    assert!(registry.tags().await.is_empty());
    assert!(registry.categories().await.is_empty());
}

#[tokio::test]
async fn test_replace_all() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("old-1", vec![], None)).await;
    registry.register(create_test_skill("old-2", vec![], None)).await;

    let new_skills = vec![
        create_test_skill("new-1", vec!["x"], None),
        create_test_skill("new-2", vec!["y"], None),
        create_test_skill("new-3", vec!["z"], None),
    ];

    registry.replace_all(new_skills).await;

    assert_eq!(registry.len().await, 3);
    assert!(registry.get("old-1").await.is_none());
    assert!(registry.get("new-1").await.is_some());
}

#[tokio::test]
async fn test_contains() {
    let registry = SkillRegistry::new();
    registry.register(create_test_skill("exists", vec![], None)).await;

    assert!(registry.contains("exists").await);
    assert!(!registry.contains("not-exists").await);
}
