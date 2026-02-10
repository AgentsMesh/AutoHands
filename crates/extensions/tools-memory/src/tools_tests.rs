use super::*;
use autohands_protocols::memory::{MemoryEntry, MemoryQuery, MemorySearchResult};
use autohands_protocols::error::MemoryError;
use std::path::PathBuf;
use std::sync::Mutex;

struct MockMemoryBackend {
    entries: Mutex<Vec<MemoryEntry>>,
}

impl MockMemoryBackend {
    fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl MemoryBackend for MockMemoryBackend {
    fn id(&self) -> &str {
        "mock"
    }

    async fn store(&self, mut entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        entry.id = Some(id.clone());
        self.entries.lock().unwrap().push(entry);
        Ok(id)
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.iter().find(|e| e.id.as_deref() == Some(id)).cloned())
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let entries = self.entries.lock().unwrap();
        let query_text = query.text.unwrap_or_default().to_lowercase();
        let results: Vec<_> = entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&query_text))
            .map(|e| MemorySearchResult {
                entry: e.clone(),
                relevance: 0.9,
            })
            .take(query.limit)
            .collect();
        Ok(results)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id.as_deref() != Some(id));
        Ok(())
    }

    async fn update(&self, _id: &str, _entry: MemoryEntry) -> Result<(), MemoryError> {
        Ok(())
    }
}

fn make_ctx() -> ToolContext {
    ToolContext::new("test-session", PathBuf::from("/tmp"))
}

#[test]
fn test_search_tool_definition() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemorySearchTool::new(backend);
    assert_eq!(tool.definition().id, "memory_search");
}

#[test]
fn test_get_tool_definition() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemoryGetTool::new(backend);
    assert_eq!(tool.definition().id, "memory_get");
}

#[test]
fn test_store_tool_definition() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemoryStoreTool::new(backend);
    assert_eq!(tool.definition().id, "memory_store");
}

#[tokio::test]
async fn test_store_and_search() {
    let backend = Arc::new(MockMemoryBackend::new());
    let store_tool = MemoryStoreTool::new(backend.clone());
    let search_tool = MemorySearchTool::new(backend.clone());

    // Store
    let params = serde_json::json!({
        "content": "User prefers Rust language",
        "memory_type": "preference",
        "importance": 0.8
    });
    let result = store_tool.execute(params, make_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("stored successfully"));

    // Search
    let params = serde_json::json!({ "query": "Rust" });
    let result = search_tool.execute(params, make_ctx()).await.unwrap();
    assert!(result.success);
    assert!(result.content.contains("Rust language"));
}

#[tokio::test]
async fn test_search_no_results() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemorySearchTool::new(backend);

    let params = serde_json::json!({ "query": "nonexistent" });
    let result = tool.execute(params, make_ctx()).await.unwrap();
    assert!(result.content.contains("No matching memories"));
}

#[tokio::test]
async fn test_get_not_found() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemoryGetTool::new(backend);

    let params = serde_json::json!({ "id": "nonexistent-id" });
    let result = tool.execute(params, make_ctx()).await.unwrap();
    assert!(result.content.contains("not found"));
}

#[tokio::test]
async fn test_store_with_tags() {
    let backend = Arc::new(MockMemoryBackend::new());
    let tool = MemoryStoreTool::new(backend);

    let params = serde_json::json!({
        "content": "Important fact",
        "tags": ["project", "rust"]
    });
    let result = tool.execute(params, make_ctx()).await.unwrap();
    assert!(result.success);
}
