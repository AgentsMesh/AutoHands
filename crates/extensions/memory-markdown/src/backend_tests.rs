use super::*;

#[tokio::test]
async fn test_backend_id() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();
    assert_eq!(backend.id(), "markdown");
}

#[tokio::test]
async fn test_store_and_retrieve() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    let entry = MemoryEntry::new("Test content", "fact")
        .with_tags(vec!["test".to_string()]);

    let id = backend.store(entry).await.unwrap();
    assert!(id.starts_with("mem_"));

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Test content");
    assert_eq!(retrieved.memory_type, "fact");
}

#[tokio::test]
async fn test_store_with_custom_id() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    let mut entry = MemoryEntry::new("Custom ID test", "fact");
    entry.id = Some("custom_id_123".to_string());

    let id = backend.store(entry).await.unwrap();
    assert_eq!(id, "custom_id_123");
}

#[tokio::test]
async fn test_delete() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    let entry = MemoryEntry::new("To delete", "temp");
    let id = backend.store(entry).await.unwrap();

    backend.delete(&id).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    let entry = MemoryEntry::new("Original", "fact");
    let id = backend.store(entry).await.unwrap();

    let updated = MemoryEntry::new("Updated content", "fact")
        .with_tags(vec!["updated".to_string()]);
    backend.update(&id, updated).await.unwrap();

    let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.content, "Updated content");
    assert!(retrieved.tags.contains(&"updated".to_string()));
}

#[tokio::test]
async fn test_search_by_type() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    backend.store(MemoryEntry::new("Fact content", "fact")).await.unwrap();
    backend.store(MemoryEntry::new("Preference content", "preference")).await.unwrap();

    let query = MemoryQuery {
        text: None,
        memory_type: Some("fact".to_string()),
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };

    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.memory_type, "fact");
}

#[tokio::test]
async fn test_search_by_text() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

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
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("fox"));
}

#[tokio::test]
async fn test_search_by_tags() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    backend.store(MemoryEntry::new("Tagged", "fact").with_tags(vec!["special".to_string()])).await.unwrap();
    backend.store(MemoryEntry::new("Untagged", "fact")).await.unwrap();

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
async fn test_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Store a memory
    {
        let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();
        backend.store(MemoryEntry::new("Persistent memory", "fact")).await.unwrap();
    }

    // Reopen and verify it's still there
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();
    let query = MemoryQuery {
        text: None,
        memory_type: None,
        tags: vec![],
        limit: 10,
        min_relevance: None,
    };
    let results = backend.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.content, "Persistent memory");
}

#[tokio::test]
async fn test_file_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MarkdownMemoryBackend::new(temp_dir.path()).await.unwrap();

    let mut entry = MemoryEntry::new("Test file format", "fact");
    entry.id = Some("test_format".to_string());
    backend.store(entry).await.unwrap();

    // Read the file directly
    let file_path = temp_dir.path().join("test_format.md");
    let content = std::fs::read_to_string(&file_path).unwrap();

    assert!(content.starts_with("---"));
    assert!(content.contains("id: test_format"));
    assert!(content.contains("type: fact"));
    assert!(content.contains("Test file format"));
}

#[test]
fn test_matches_text() {
    let memory = MarkdownMemory::new("test", "fact", "The quick brown fox jumps over the lazy dog");

    // Exact match
    let score = MarkdownMemoryBackend::matches_text(&memory, "fox");
    assert!(score > 0.0);

    // No match
    let score = MarkdownMemoryBackend::matches_text(&memory, "cat");
    assert_eq!(score, 0.0);

    // Case insensitive
    let score = MarkdownMemoryBackend::matches_text(&memory, "FOX");
    assert!(score > 0.0);
}
