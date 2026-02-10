//! Memory tool implementations: search, get, store.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::memory::{MemoryBackend, MemoryEntry, MemoryQuery};
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

// ---------------------------------------------------------------------------
// memory_search
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MemorySearchParams {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    min_relevance: Option<f32>,
    #[serde(default)]
    memory_type: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

/// Semantic search over the memory store.
pub struct MemorySearchTool {
    definition: ToolDefinition,
    backend: Arc<dyn MemoryBackend>,
}

impl MemorySearchTool {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query text"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default 10)"
                },
                "min_relevance": {
                    "type": "number",
                    "description": "Minimum relevance score 0.0-1.0"
                },
                "memory_type": {
                    "type": "string",
                    "description": "Filter by memory type (fact, decision, preference, todo, conversation)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                }
            },
            "required": ["query"]
        });

        Self {
            definition: ToolDefinition::new(
                "memory_search",
                "Memory Search",
                "Search long-term memory for relevant information. Use this to recall past conversations, user preferences, decisions, and facts.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            backend,
        }
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MemorySearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let query = MemoryQuery {
            text: Some(params.query.clone()),
            memory_type: params.memory_type,
            tags: params.tags.unwrap_or_default(),
            limit: params.limit.unwrap_or(10),
            min_relevance: params.min_relevance,
        };

        debug!("memory_search: query={:?}", params.query);

        let results = self
            .backend
            .search(query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Memory search failed: {}", e)))?;

        if results.is_empty() {
            return Ok(ToolResult::success("No matching memories found."));
        }

        let mut output = format!("Found {} matching memories:\n\n", results.len());
        for (i, result) in results.iter().enumerate() {
            let entry = &result.entry;
            let id = entry.id.as_deref().unwrap_or("unknown");
            let created = entry
                .created_at
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let importance = entry
                .importance
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "-".to_string());
            let tags = if entry.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", entry.tags.join(", "))
            };

            output.push_str(&format!(
                "---\n#{} (id: {}, type: {}, relevance: {:.2}, importance: {}, created: {}{})\n{}\n",
                i + 1,
                id,
                entry.memory_type,
                result.relevance,
                importance,
                created,
                tags,
                entry.content,
            ));
        }

        Ok(ToolResult::success(output))
    }
}

// ---------------------------------------------------------------------------
// memory_get
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MemoryGetParams {
    id: String,
}

/// Retrieve a single memory entry by ID.
pub struct MemoryGetTool {
    definition: ToolDefinition,
    backend: Arc<dyn MemoryBackend>,
}

impl MemoryGetTool {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Memory entry ID"
                }
            },
            "required": ["id"]
        });

        Self {
            definition: ToolDefinition::new(
                "memory_get",
                "Memory Get",
                "Retrieve a specific memory entry by its ID.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            backend,
        }
    }
}

#[async_trait]
impl Tool for MemoryGetTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MemoryGetParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        debug!("memory_get: id={}", params.id);

        let entry = self
            .backend
            .retrieve(&params.id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Memory retrieve failed: {}", e)))?;

        match entry {
            Some(entry) => {
                let json = serde_json::to_string_pretty(&entry)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                Ok(ToolResult::success(json))
            }
            None => Ok(ToolResult::success(format!(
                "Memory entry not found: {}",
                params.id
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// memory_store
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MemoryStoreParams {
    content: String,
    #[serde(default = "default_memory_type")]
    memory_type: String,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    importance: Option<f32>,
}

fn default_memory_type() -> String {
    "fact".to_string()
}

/// Store a new memory entry.
pub struct MemoryStoreTool {
    definition: ToolDefinition,
    backend: Arc<dyn MemoryBackend>,
}

impl MemoryStoreTool {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Content to remember"
                },
                "memory_type": {
                    "type": "string",
                    "description": "Type of memory: fact, decision, preference, todo, conversation (default: fact)",
                    "enum": ["fact", "decision", "preference", "todo", "conversation"]
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Tags for categorization"
                },
                "importance": {
                    "type": "number",
                    "description": "Importance score 0.0-1.0 (higher = more important)"
                }
            },
            "required": ["content"]
        });

        Self {
            definition: ToolDefinition::new(
                "memory_store",
                "Memory Store",
                "Store important information in long-term memory. Use this to remember user preferences, key decisions, facts, and action items.",
            )
            .with_parameters_schema(schema)
            .with_risk_level(RiskLevel::Low),
            backend,
        }
    }
}

#[async_trait]
impl Tool for MemoryStoreTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MemoryStoreParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        debug!(
            "memory_store: type={}, content_len={}",
            params.memory_type,
            params.content.len()
        );

        let mut entry = MemoryEntry::new(&params.content, &params.memory_type);
        if let Some(tags) = params.tags {
            entry = entry.with_tags(tags);
        }
        if let Some(importance) = params.importance {
            entry = entry.with_importance(importance);
        }

        let id = self
            .backend
            .store(entry)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Memory store failed: {}", e)))?;

        Ok(ToolResult::success(format!(
            "Memory stored successfully (id: {})",
            id
        )))
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
