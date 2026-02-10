use super::*;

fn create_backend() -> VectorMemoryBackend {
    VectorMemoryBackend::with_simple_embedding("test")
}

#[tokio::test]
async fn test_store_and_retrieve() {
    let backend = create_backend();

    let entry = MemoryEntry::new("Hello world", "fact");
    let id = backend.store(entry).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Hello world");
    assert_eq!(retrieved.memory_type, "fact");
}

#[tokio::test]
async fn test_delete() {
    let backend = create_backend();

    let entry = MemoryEntry::new("Test content", "fact");
    let id = backend.store(entry).await.unwrap();

    backend.delete(&id).await.unwrap();
    assert!(backend.retrieve(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_update() {
    let backend = create_backend();

    let entry = MemoryEntry::new("Original", "fact");
    let id = backend.store(entry).await.unwrap();

    let updated = MemoryEntry::new("Updated", "fact");
    backend.update(&id, updated).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Updated");
}

#[tokio::test]
async fn test_semantic_search() {
    let backend = create_backend();

    // Store some entries
    backend
        .store(MemoryEntry::new("The cat sat on the mat", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Dogs are friendly animals", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Cats are independent creatures", "fact"))
        .await
        .unwrap();

    // Search for cat-related content
    let query = MemoryQuery::text("cat").with_limit(2);
    let results = backend.search(query).await.unwrap();

    assert!(!results.is_empty());
    assert!(results.len() <= 2);
}

#[tokio::test]
async fn test_search_with_type_filter() {
    let backend = create_backend();

    backend
        .store(MemoryEntry::new("Fact about cats", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("User prefers cats", "preference"))
        .await
        .unwrap();

    let mut query = MemoryQuery::text("cats").with_limit(10);
    query.memory_type = Some("fact".to_string());

    let results = backend.search(query).await.unwrap();
    assert!(results.iter().all(|r| r.entry.memory_type == "fact"));
}

#[tokio::test]
async fn test_search_with_tags_filter() {
    let backend = create_backend();

    backend
        .store(MemoryEntry::new("Tagged entry", "fact").with_tags(vec!["important".into()]))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Untagged entry", "fact"))
        .await
        .unwrap();

    let mut query = MemoryQuery::text("entry").with_limit(10);
    query.tags = vec!["important".to_string()];

    let results = backend.search(query).await.unwrap();
    assert!(results.iter().all(|r| r.entry.tags.contains(&"important".to_string())));
}

#[test]
fn test_backend_id() {
    let backend = create_backend();
    assert_eq!(backend.id(), "test");
}

#[tokio::test]
async fn test_store_with_custom_id() {
    let backend = create_backend();
    let mut entry = MemoryEntry::new("Test", "fact");
    entry.id = Some("custom-id".to_string());

    let id = backend.store(entry).await.unwrap();
    assert_eq!(id, "custom-id");
}

#[tokio::test]
async fn test_store_with_created_at() {
    let backend = create_backend();
    let mut entry = MemoryEntry::new("Test", "fact");
    let fixed_time = chrono::Utc::now() - chrono::Duration::hours(1);
    entry.created_at = Some(fixed_time);

    let id = backend.store(entry).await.unwrap();
    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.created_at, Some(fixed_time));
}

#[tokio::test]
async fn test_retrieve_nonexistent() {
    let backend = create_backend();
    let result = backend.retrieve("nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_search_without_text_query() {
    let backend = create_backend();

    backend.store(MemoryEntry::new("Entry 1", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Entry 2", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Entry 3", "fact")).await.unwrap();

    let query = MemoryQuery {
        text: None,
        memory_type: None,
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_search_with_limit() {
    let backend = create_backend();

    for i in 0..5 {
        backend.store(MemoryEntry::new(format!("Entry {}", i), "fact")).await.unwrap();
    }

    let query = MemoryQuery::text("Entry").with_limit(2);
    let results = backend.search(query).await.unwrap();
    assert!(results.len() <= 2);
}

#[tokio::test]
async fn test_search_with_min_relevance() {
    let backend = create_backend();

    backend.store(MemoryEntry::new("Highly relevant content about cats", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Something completely different", "fact")).await.unwrap();

    let mut query = MemoryQuery::text("cats").with_limit(10);
    query.min_relevance = Some(0.1);

    let results = backend.search(query).await.unwrap();
    // Results should be filtered by relevance
    assert!(results.iter().all(|r| r.relevance >= 0.1));
}

#[tokio::test]
async fn test_backend_new_with_custom_embedder() {
    let embedder = Arc::new(SimpleHashEmbedding::new(64));
    let backend = VectorMemoryBackend::new("custom", embedder);
    assert_eq!(backend.id(), "custom");
}

#[tokio::test]
async fn test_delete_nonexistent() {
    let backend = create_backend();
    // Should not error when deleting non-existent entry
    let result = backend.delete("nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_nonexistent() {
    let backend = create_backend();
    // Update creates entry if it doesn't exist
    let entry = MemoryEntry::new("New content", "fact");
    let result = backend.update("new-id", entry).await;
    assert!(result.is_ok());

    let retrieved = backend.retrieve("new-id").await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_search_sorts_by_relevance() {
    let backend = create_backend();

    backend.store(MemoryEntry::new("cats cats cats", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("dogs are cute", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("cats are independent", "fact")).await.unwrap();

    let query = MemoryQuery::text("cats").with_limit(10);
    let results = backend.search(query).await.unwrap();

    // Check that results are sorted by relevance (descending)
    for i in 1..results.len() {
        assert!(results[i - 1].relevance >= results[i].relevance);
    }
}
