//! Code analysis tools.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::analyzer::{detect_language, FileAnalysis, PatternAnalyzer};

/// Parameters for code analysis.
#[derive(Debug, Deserialize)]
pub struct AnalyzeCodeParams {
    pub path: String,
    #[serde(default)]
    pub include_signatures: bool,
}

/// Code analyzer tool.
pub struct AnalyzeCodeTool {
    definition: ToolDefinition,
}

impl AnalyzeCodeTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "analyze_code",
                "Analyze Code",
                "Analyze source code to extract functions, classes, structs, and other elements",
            ),
        }
    }
}

impl Default for AnalyzeCodeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AnalyzeCodeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AnalyzeCodeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let path = ctx.work_dir.join(&params.path);
        debug!("Analyzing code: {:?}", path);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                params.path
            )));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let language = detect_language(&path).unwrap_or_else(|| "unknown".to_string());

        let elements = if language == "rust" {
            PatternAnalyzer::analyze_rust(&content)
        } else {
            Vec::new()
        };

        let analysis = FileAnalysis {
            path: params.path,
            language,
            elements,
            imports: Vec::new(),
            line_count: content.lines().count(),
        };

        let result = serde_json::to_string_pretty(&analysis)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(result))
    }
}

/// Parameters for symbol search.
#[derive(Debug, Deserialize)]
pub struct FindSymbolParams {
    pub symbol: String,
    pub path: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

/// Symbol search result.
#[derive(Debug, Serialize)]
pub struct SymbolMatch {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub element_type: Option<String>,
}

/// Find symbol tool.
pub struct FindSymbolTool {
    definition: ToolDefinition,
}

impl FindSymbolTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "find_symbol",
                "Find Symbol",
                "Search for a symbol (function, class, variable) in the codebase",
            ),
        }
    }
}

impl Default for FindSymbolTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FindSymbolTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: FindSymbolParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let path = ctx.work_dir.join(&params.path);
        debug!("Finding symbol '{}' in {:?}", params.symbol, path);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path not found: {}",
                params.path
            )));
        }

        let mut matches = Vec::new();

        if path.is_file() {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            for (i, line) in content.lines().enumerate() {
                let found = if params.case_sensitive {
                    line.contains(&params.symbol)
                } else {
                    line.to_lowercase()
                        .contains(&params.symbol.to_lowercase())
                };

                if found {
                    matches.push(SymbolMatch {
                        file: params.path.clone(),
                        line: i + 1,
                        content: line.trim().to_string(),
                        element_type: None,
                    });
                }
            }
        }

        let result = serde_json::to_string_pretty(&matches)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(result))
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
