//! Content search (grep) tool.

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

#[derive(Debug, Deserialize)]
struct GrepParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default = "default_context")]
    context: usize,
    #[serde(default)]
    case_insensitive: bool,
}

fn default_context() -> usize {
    0
}

/// Content search tool.
pub struct GrepTool {
    definition: ToolDefinition,
}

impl GrepTool {
    pub fn new() -> Self {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "glob": {
                    "type": "string",
                    "description": "File pattern filter (e.g., *.rs)"
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context around matches"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                }
            },
            "required": ["pattern"]
        });

        Self {
            definition: ToolDefinition::new("grep", "Content Search", "Search file contents")
                .with_parameters_schema(schema)
                .with_risk_level(RiskLevel::Low),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GrepParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let pattern = if params.case_insensitive {
            format!("(?i){}", params.pattern)
        } else {
            params.pattern.clone()
        };

        let regex = Regex::new(&pattern)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid regex: {}", e)))?;

        let search_path = params
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.work_dir.clone());

        let mut results = Vec::new();
        let glob_pattern = params.glob.as_deref();

        for entry in WalkDir::new(&search_path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            if let Some(glob) = glob_pattern {
                if !matches_glob(entry.path(), glob) {
                    continue;
                }
            }

            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                let matches = search_file(&content, &regex, params.context);
                if !matches.is_empty() {
                    let rel_path = entry.path()
                        .strip_prefix(&search_path)
                        .unwrap_or(entry.path());
                    results.push(format!("{}:\n{}", rel_path.display(), matches.join("\n")));
                }
            }
        }

        if results.is_empty() {
            Ok(ToolResult::success("No matches found"))
        } else {
            Ok(ToolResult::success(results.join("\n\n")))
        }
    }
}

fn matches_glob(path: &std::path::Path, pattern: &str) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    glob::Pattern::new(pattern)
        .map(|p| p.matches(file_name))
        .unwrap_or(false)
}

fn search_file(content: &str, regex: &Regex, context: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            let start = i.saturating_sub(context);
            let end = (i + context + 1).min(lines.len());
            let snippet: Vec<String> = lines[start..end]
                .iter()
                .enumerate()
                .map(|(j, l)| format!("{:>4}: {}", start + j + 1, l))
                .collect();
            matches.push(snippet.join("\n"));
        }
    }

    matches
}

#[cfg(test)]
#[path = "grep_tool_tests.rs"]
mod tests;
