use super::*;

#[tokio::test]
async fn test_backend_id() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    assert_eq!(backend.id(), "sqlite");
}

#[tokio::test]
async fn test_store_and_retrieve() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let entry = MemoryEntry::new("Test content", "fact").with_tags(vec!["test".to_string()]);

    let id = backend.store(entry).await.unwrap();
    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

    assert_eq!(retrieved.content, "Test content");
    assert_eq!(retrieved.memory_type, "fact");
    assert!(retrieved.tags.contains(&"test".to_string()));
}

#[tokio::test]
async fn test_store_with_id() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let mut entry = MemoryEntry::new("Test", "fact");
    entry.id = Some("custom-id-123".to_string());

    let id = backend.store(entry).await.unwrap();
    assert_eq!(id, "custom-id-123");

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, Some("custom-id-123".to_string()));
}

#[tokio::test]
async fn test_store_with_importance() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let entry = MemoryEntry::new("Important fact", "fact").with_importance(0.9);

    let id = backend.store(entry).await.unwrap();
    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

    assert_eq!(retrieved.importance, Some(0.9));
}

#[tokio::test]
async fn test_store_with_metadata() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let mut entry = MemoryEntry::new("Test", "fact");
    entry.metadata.insert("key".to_string(), serde_json::json!("value"));

    let id = backend.store(entry).await.unwrap();
    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

    assert_eq!(
        retrieved.metadata.get("key"),
        Some(&serde_json::json!("value"))
    );
}

#[tokio::test]
async fn test_retrieve_nonexistent() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let result = backend.retrieve("nonexistent-id").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let entry = MemoryEntry::new("To delete", "temp");

    let id = backend.store(entry).await.unwrap();
    backend.delete(&id).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    // Should not error
    let result = backend.delete("nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();
    let entry = MemoryEntry::new("Original", "fact");

    let id = backend.store(entry).await.unwrap();

    let updated = MemoryEntry::new("Updated content", "fact")
        .with_tags(vec!["new-tag".to_string()]);
    backend.update(&id, updated).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Updated content");
    assert!(retrieved.tags.contains(&"new-tag".to_string()));
}

#[tokio::test]
async fn test_search_basic() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();

    backend.store(MemoryEntry::new("Rust programming", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Python scripting", "fact")).await.unwrap();

    let query = MemoryQuery {
        text: None,
        memory_type: Some("fact".to_string()),
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_search_with_fts() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();

    backend.store(MemoryEntry::new("The quick brown fox", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("A lazy dog sleeps", "fact")).await.unwrap();

    let query = MemoryQuery {
        text: Some("fox".to_string()),
        memory_type: None,
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert!(!results.is_empty());
    assert!(results[0].entry.content.contains("fox"));
}

#[tokio::test]
async fn test_search_by_type() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();

    backend.store(MemoryEntry::new("Fact content", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Preference content", "preference")).await.unwrap();

    let query = MemoryQuery {
        text: None,
        memory_type: Some("preference".to_string()),
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.memory_type, "preference");
}

#[tokio::test]
async fn test_search_limit() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();

    for i in 0..5 {
        backend.store(MemoryEntry::new(format!("Entry {}", i), "fact")).await.unwrap();
    }

    let query = MemoryQuery {
        text: None,
        memory_type: None,
        tags: vec![],
        limit: 2,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_search_by_tags() {
    let backend = SqliteMemoryBackend::in_memory().await.unwrap();

    backend.store(MemoryEntry::new("Tagged entry", "fact").with_tags(vec!["special".to_string()])).await.unwrap();
    backend.store(MemoryEntry::new("Regular entry", "fact")).await.unwrap();

    let query = MemoryQuery {
        text: None,
        memory_type: None,
        tags: vec!["special".to_string()],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_file_backend() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    {
        let backend = SqliteMemoryBackend::open(&db_path).await.unwrap();
        backend.store(MemoryEntry::new("Persistent", "fact")).await.unwrap();
    }

    // Reopen and verify
    let backend = SqliteMemoryBackend::open(&db_path).await.unwrap();
    let query = MemoryQuery {
        text: None,
        memory_type: None,
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };
    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
}
