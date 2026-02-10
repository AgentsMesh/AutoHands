use super::*;
use autohands_memory_vector::SimpleHashEmbedding;

async fn create_test_backend() -> HybridMemoryBackend {
    let embedder = Arc::new(SimpleHashEmbedding::default());
    HybridMemoryBackend::new("test", embedder, HybridMemoryConfig::default())
        .await
        .unwrap()
}

#[tokio::test]
async fn test_hybrid_backend_creation() {
    let backend = create_test_backend().await;
    assert_eq!(backend.id(), "test");
}

#[tokio::test]
async fn test_store_and_retrieve() {
    let backend = create_test_backend().await;

    let entry = MemoryEntry::new("Test content about Rust programming", "fact");
    let id = backend.store(entry).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().content.contains("Rust"));
}

#[tokio::test]
async fn test_hybrid_search() {
    let backend = create_test_backend().await;

    backend
        .store(MemoryEntry::new("Rust programming language is fast", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Python is a scripting language", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Rust has zero-cost abstractions", "fact"))
        .await
        .unwrap();

    let query = MemoryQuery::text("Rust programming").with_limit(10);
    let results = backend.search(query).await.unwrap();

    // Should find Rust-related entries
    assert!(!results.is_empty());
    assert!(results
        .iter()
        .any(|r| r.entry.content.contains("Rust")));
}

#[tokio::test]
async fn test_delete() {
    let backend = create_test_backend().await;

    let entry = MemoryEntry::new("To be deleted", "fact");
    let id = backend.store(entry).await.unwrap();

    backend.delete(&id).await.unwrap();
    assert!(backend.retrieve(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_update() {
    let backend = create_test_backend().await;

    let entry = MemoryEntry::new("Original", "fact");
    let id = backend.store(entry).await.unwrap();

    let updated = MemoryEntry::new("Updated content", "fact");
    backend.update(&id, updated).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Updated content");
}

#[tokio::test]
async fn test_search_with_type_filter() {
    let backend = create_test_backend().await;

    backend
        .store(MemoryEntry::new("Fact entry", "fact"))
        .await
        .unwrap();
    backend
        .store(MemoryEntry::new("Preference entry", "preference"))
        .await
        .unwrap();

    let mut query = MemoryQuery::text("entry").with_limit(10);
    query.memory_type = Some("fact".to_string());

    let results = backend.search(query).await.unwrap();
    assert!(results.iter().all(|r| r.entry.memory_type == "fact"));
}

#[test]
fn test_config_default() {
    let config = HybridMemoryConfig::default();
    assert!((config.min_relevance - 0.0).abs() < 0.01);
}
