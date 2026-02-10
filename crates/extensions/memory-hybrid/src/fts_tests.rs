use super::*;

#[tokio::test]
async fn test_fts_backend_creation() {
    let backend = FTSBackend::new().await.unwrap();
    assert!(backend.entries.read().is_empty());
}

#[tokio::test]
async fn test_fts_index_and_search() {
    let backend = FTSBackend::new().await.unwrap();

    let mut entry = MemoryEntry::new("Hello world test content", "fact");
    entry.id = Some("entry-1".to_string());

    backend.index(&entry).await.unwrap();

    let results = backend.search("hello", 10).await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, "entry-1");
}

#[tokio::test]
async fn test_fts_remove() {
    let backend = FTSBackend::new().await.unwrap();

    let mut entry = MemoryEntry::new("Test content", "fact");
    entry.id = Some("entry-1".to_string());

    backend.index(&entry).await.unwrap();
    backend.remove("entry-1").await.unwrap();

    let results = backend.search("test", 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_fts_empty_query() {
    let backend = FTSBackend::new().await.unwrap();
    let results = backend.search("", 10).await.unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_escape_fts_query() {
    let escaped = escape_fts_query("hello world");
    assert!(escaped.contains("hello"));
    assert!(escaped.contains("world"));
    assert!(escaped.contains("OR"));
}

#[test]
fn test_escape_fts_query_with_quotes() {
    let escaped = escape_fts_query("hello \"world\"");
    // Quotes should be stripped
    assert!(!escaped.contains("\\\""));
}

#[tokio::test]
async fn test_get_entry() {
    let backend = FTSBackend::new().await.unwrap();

    let mut entry = MemoryEntry::new("Test", "fact");
    entry.id = Some("entry-1".to_string());

    backend.index(&entry).await.unwrap();

    let retrieved = backend.get_entry("entry-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, "Test");
}

#[tokio::test]
async fn test_fts_multiple_entries() {
    let backend = FTSBackend::new().await.unwrap();

    for i in 0..5 {
        let mut entry = MemoryEntry::new(format!("Entry {} content about Rust", i), "fact");
        entry.id = Some(format!("entry-{}", i));
        backend.index(&entry).await.unwrap();
    }

    let results = backend.search("Rust", 10).await.unwrap();
    assert_eq!(results.len(), 5);
}
