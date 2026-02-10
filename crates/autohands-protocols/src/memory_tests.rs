use super::*;

#[test]
fn test_memory_entry_new() {
    let entry = MemoryEntry::new("Test content", "fact");
    assert_eq!(entry.content, "Test content");
    assert_eq!(entry.memory_type, "fact");
    assert!(entry.id.is_none());
    assert!(entry.tags.is_empty());
    assert!(entry.created_at.is_some());
}

#[test]
fn test_memory_entry_with_tags() {
    let entry = MemoryEntry::new("Test", "fact")
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
    assert_eq!(entry.tags.len(), 2);
    assert!(entry.tags.contains(&"tag1".to_string()));
}

#[test]
fn test_memory_entry_with_importance() {
    let entry = MemoryEntry::new("Test", "fact")
        .with_importance(0.8);
    assert_eq!(entry.importance, Some(0.8));
}

#[test]
fn test_memory_entry_importance_clamped() {
    let entry1 = MemoryEntry::new("Test", "fact").with_importance(1.5);
    let entry2 = MemoryEntry::new("Test", "fact").with_importance(-0.5);
    assert_eq!(entry1.importance, Some(1.0));
    assert_eq!(entry2.importance, Some(0.0));
}

#[test]
fn test_memory_entry_serialization() {
    let entry = MemoryEntry::new("Test content", "conversation");
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("Test content"));
    assert!(json.contains("conversation"));
}

#[test]
fn test_memory_entry_deserialization() {
    let json = r#"{"content":"Test","memory_type":"fact"}"#;
    let entry: MemoryEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.content, "Test");
    assert_eq!(entry.memory_type, "fact");
}

#[test]
fn test_memory_entry_clone() {
    let entry = MemoryEntry::new("Test", "fact")
        .with_tags(vec!["tag".to_string()])
        .with_importance(0.5);
    let cloned = entry.clone();
    assert_eq!(cloned.content, entry.content);
    assert_eq!(cloned.tags, entry.tags);
    assert_eq!(cloned.importance, entry.importance);
}

#[test]
fn test_memory_query_text() {
    let query = MemoryQuery::text("search term");
    assert_eq!(query.text, Some("search term".to_string()));
    assert_eq!(query.limit, 10);
}

#[test]
fn test_memory_query_with_limit() {
    let query = MemoryQuery::text("test").with_limit(20);
    assert_eq!(query.limit, 20);
}

#[test]
fn test_memory_query_default() {
    let query = MemoryQuery::default();
    assert!(query.text.is_none());
    assert!(query.memory_type.is_none());
    assert!(query.tags.is_empty());
    assert_eq!(query.limit, 0);
}

#[test]
fn test_memory_query_serialization() {
    let query = MemoryQuery::text("test").with_limit(5);
    let json = serde_json::to_string(&query).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("5"));
}

#[test]
fn test_memory_query_deserialization() {
    let json = r#"{"text":"search","limit":10}"#;
    let query: MemoryQuery = serde_json::from_str(json).unwrap();
    assert_eq!(query.text, Some("search".to_string()));
    assert_eq!(query.limit, 10);
}

#[test]
fn test_memory_search_result_serialization() {
    let entry = MemoryEntry::new("Test", "fact");
    let result = MemorySearchResult {
        entry,
        relevance: 0.95,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("0.95"));
}

#[test]
fn test_memory_search_result_clone() {
    let entry = MemoryEntry::new("Test", "fact");
    let result = MemorySearchResult {
        entry,
        relevance: 0.8,
    };
    let cloned = result.clone();
    assert_eq!(cloned.relevance, result.relevance);
}

#[test]
fn test_memory_entry_debug() {
    let entry = MemoryEntry::new("Test", "fact");
    let debug = format!("{:?}", entry);
    assert!(debug.contains("MemoryEntry"));
}

#[test]
fn test_memory_query_debug() {
    let query = MemoryQuery::text("test");
    let debug = format!("{:?}", query);
    assert!(debug.contains("MemoryQuery"));
}

#[test]
fn test_memory_search_result_debug() {
    let entry = MemoryEntry::new("Test", "fact");
    let result = MemorySearchResult {
        entry,
        relevance: 0.5,
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("MemorySearchResult"));
}

#[test]
fn test_memory_query_with_all_fields() {
    let query = MemoryQuery {
        text: Some("search".to_string()),
        memory_type: Some("fact".to_string()),
        tags: vec!["tag1".to_string()],
        limit: 15,
        min_relevance: Some(0.5),
    };
    let json = serde_json::to_string(&query).unwrap();
    assert!(json.contains("search"));
    assert!(json.contains("fact"));
    assert!(json.contains("tag1"));
}
