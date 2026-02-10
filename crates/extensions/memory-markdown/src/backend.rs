//! Markdown memory backend implementation.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};
use walkdir::WalkDir;

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{MemoryBackend, MemoryEntry, MemoryQuery, MemorySearchResult};

use crate::error::MarkdownMemoryError;
use crate::parser::{MarkdownMemory, MarkdownParser};

/// Markdown-based memory backend.
///
/// Stores memories as individual Markdown files with YAML front matter.
pub struct MarkdownMemoryBackend {
    storage_path: PathBuf,
    /// In-memory cache of all memories for fast search.
    cache: Arc<RwLock<HashMap<String, MarkdownMemory>>>,
}

impl MarkdownMemoryBackend {
    /// Create a new Markdown memory backend.
    pub async fn new(storage_path: impl AsRef<Path>) -> Result<Self, MarkdownMemoryError> {
        let storage_path = storage_path.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path).await.map_err(|e| {
                MarkdownMemoryError::CreateDirFailed {
                    path: storage_path.clone(),
                    reason: e.to_string(),
                }
            })?;
            info!("Created memory storage directory: {:?}", storage_path);
        }

        let backend = Self {
            storage_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing memories into cache
        backend.load_all_to_cache().await?;

        Ok(backend)
    }

    /// Create a backend with default storage path (~/.autohands/memory/).
    pub async fn default_path() -> Result<Self, MarkdownMemoryError> {
        let home = dirs::home_dir().ok_or(MarkdownMemoryError::StoragePathNotSet)?;
        let storage_path = home.join(".autohands").join("memory");
        Self::new(storage_path).await
    }

    /// Load all memories from disk into cache.
    async fn load_all_to_cache(&self) -> Result<(), MarkdownMemoryError> {
        let mut cache = self.cache.write().await;
        cache.clear();

        let storage_path = self.storage_path.clone();

        // Use blocking task for walkdir
        let entries: Vec<(String, MarkdownMemory)> = tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();

            for entry in WalkDir::new(&storage_path)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "md") {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(memory) = MarkdownParser::parse(&content) {
                            results.push((memory.front_matter.id.clone(), memory));
                        }
                    }
                }
            }

            results
        })
        .await
        .map_err(|e| MarkdownMemoryError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )))?;

        for (id, memory) in entries {
            cache.insert(id, memory);
        }

        info!("Loaded {} memories from disk", cache.len());
        Ok(())
    }

    /// Get the file path for a memory ID.
    fn memory_path(&self, id: &str) -> PathBuf {
        self.storage_path.join(MarkdownParser::id_to_filename(id))
    }

    /// Save a memory to disk.
    async fn save_to_disk(&self, memory: &MarkdownMemory) -> Result<(), MarkdownMemoryError> {
        let path = self.memory_path(&memory.front_matter.id);
        let content = memory.to_markdown()?;
        fs::write(&path, content).await?;
        debug!("Saved memory to {:?}", path);
        Ok(())
    }

    /// Delete a memory from disk.
    async fn delete_from_disk(&self, id: &str) -> Result<(), MarkdownMemoryError> {
        let path = self.memory_path(id);
        if path.exists() {
            fs::remove_file(&path).await?;
            debug!("Deleted memory file {:?}", path);
        }
        Ok(())
    }

    /// Simple text search in content.
    fn matches_text(memory: &MarkdownMemory, text: &str) -> f32 {
        let text_lower = text.to_lowercase();
        let content_lower = memory.content.to_lowercase();

        if content_lower.contains(&text_lower) {
            // Calculate a simple relevance score
            let occurrences = content_lower.matches(&text_lower).count();
            let content_len = content_lower.len().max(1);
            // Normalize: more occurrences and shorter content = higher relevance
            (occurrences as f32 / (content_len as f32 / 100.0)).min(1.0)
        } else {
            0.0
        }
    }
}

#[async_trait]
impl MemoryBackend for MarkdownMemoryBackend {
    fn id(&self) -> &str {
        "markdown"
    }

    async fn store(&self, entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = entry.id.clone().unwrap_or_else(|| {
            format!("mem_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string())
        });

        let memory = MarkdownMemory {
            front_matter: crate::parser::FrontMatter {
                id: id.clone(),
                memory_type: entry.memory_type,
                tags: entry.tags,
                importance: entry.importance,
                created: entry.created_at.unwrap_or_else(Utc::now),
                updated: Some(Utc::now()),
                metadata: entry.metadata,
            },
            content: entry.content,
        };

        // Save to disk
        self.save_to_disk(&memory)
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))?;

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(id.clone(), memory);

        Ok(id)
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let cache = self.cache.read().await;

        Ok(cache.get(id).map(|memory| MemoryEntry {
            id: Some(memory.front_matter.id.clone()),
            content: memory.content.clone(),
            memory_type: memory.front_matter.memory_type.clone(),
            tags: memory.front_matter.tags.clone(),
            created_at: Some(memory.front_matter.created),
            importance: memory.front_matter.importance,
            metadata: memory.front_matter.metadata.clone(),
        }))
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let cache = self.cache.read().await;
        let mut results: Vec<MemorySearchResult> = Vec::new();

        for memory in cache.values() {
            // Filter by type
            if let Some(ref mem_type) = query.memory_type {
                if &memory.front_matter.memory_type != mem_type {
                    continue;
                }
            }

            // Filter by tags
            if !query.tags.is_empty() {
                let has_tag = query
                    .tags
                    .iter()
                    .any(|t| memory.front_matter.tags.contains(t));
                if !has_tag {
                    continue;
                }
            }

            // Calculate relevance
            let relevance = if let Some(ref text) = query.text {
                let score = Self::matches_text(memory, text);
                if score == 0.0 {
                    continue;
                }
                score
            } else {
                // No text query, use importance or default
                memory.front_matter.importance.unwrap_or(0.5)
            };

            // Filter by min relevance
            if let Some(min_rel) = query.min_relevance {
                if relevance < min_rel {
                    continue;
                }
            }

            results.push(MemorySearchResult {
                entry: MemoryEntry {
                    id: Some(memory.front_matter.id.clone()),
                    content: memory.content.clone(),
                    memory_type: memory.front_matter.memory_type.clone(),
                    tags: memory.front_matter.tags.clone(),
                    created_at: Some(memory.front_matter.created),
                    importance: memory.front_matter.importance,
                    metadata: memory.front_matter.metadata.clone(),
                },
                relevance,
            });
        }

        // Sort by relevance (descending)
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        // Delete from disk
        self.delete_from_disk(id)
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))?;

        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(id);

        Ok(())
    }

    async fn update(&self, id: &str, entry: MemoryEntry) -> Result<(), MemoryError> {
        let mut cache = self.cache.write().await;

        if let Some(existing) = cache.get_mut(id) {
            existing.content = entry.content;
            existing.front_matter.memory_type = entry.memory_type;
            existing.front_matter.tags = entry.tags;
            existing.front_matter.importance = entry.importance;
            existing.front_matter.updated = Some(Utc::now());
            existing.front_matter.metadata = entry.metadata;

            // Save to disk
            let memory_clone = existing.clone();
            drop(cache); // Release lock before async operation

            self.save_to_disk(&memory_clone)
                .await
                .map_err(|e| MemoryError::QueryError(e.to_string()))?;
        } else {
            return Err(MemoryError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
