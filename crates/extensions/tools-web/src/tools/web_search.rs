//! Web search tool implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for web_search tool.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// Search query.
    query: String,

    /// Maximum number of results.
    #[serde(default = "default_max_results")]
    max_results: u32,
}

fn default_max_results() -> u32 {
    10
}

/// A search result.
#[derive(Debug, Serialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// Tool for web search.
/// Note: In production, this would integrate with a search API.
pub struct WebSearchTool {
    definition: ToolDefinition,
}

impl WebSearchTool {
    /// Create a new web search tool.
    pub fn new() -> Self {
        let definition = ToolDefinition::new(
            "web_search",
            "Web Search",
            "Search the web for information",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 10
                }
            },
            "required": ["query"]
        }))
        .with_risk_level(RiskLevel::Low);

        Self { definition }
    }

    /// Perform the search (stub implementation).
    async fn search(&self, query: &str, max_results: u32) -> Vec<SearchResult> {
        // This is a stub. In production, integrate with:
        // - Google Custom Search API
        // - Bing Search API
        // - DuckDuckGo API
        // - SerpAPI
        // etc.

        // Return mock results for demonstration
        vec![
            SearchResult {
                title: format!("Result 1 for: {}", query),
                url: format!("https://example.com/1?q={}", urlencoding::encode(query)),
                snippet: format!("This is a mock search result for '{}'", query),
            },
            SearchResult {
                title: format!("Result 2 for: {}", query),
                url: format!("https://example.com/2?q={}", urlencoding::encode(query)),
                snippet: format!("Another mock result about '{}'", query),
            },
        ]
        .into_iter()
        .take(max_results as usize)
        .collect()
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        tracing::warn!("web_search is using stub implementation, results are mock data");

        let params: SearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        if params.query.trim().is_empty() {
            return Err(ToolError::InvalidParameters("Query cannot be empty".to_string()));
        }

        let results = self.search(&params.query, params.max_results).await;

        let output = serde_json::json!({
            "query": params.query,
            "results": results,
            "count": results.len()
        });

        Ok(ToolResult::success(serde_json::to_string_pretty(&output).unwrap()))
    }
}

/// URL encoding helper.
mod urlencoding {
    pub fn encode(input: &str) -> String {
        url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_tool_definition() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.definition().id, "web_search");
        assert_eq!(tool.definition().risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_search() {
        let tool = WebSearchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "query": "rust programming"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("rust programming"));
        assert!(result.content.contains("results"));
    }

    #[tokio::test]
    async fn test_search_max_results() {
        let tool = WebSearchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "query": "test",
            "max_results": 1
        });

        let result = tool.execute(params, ctx).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(parsed["count"], 1);
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let tool = WebSearchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "query": "   "
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello+world");
        assert_eq!(urlencoding::encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }
}
