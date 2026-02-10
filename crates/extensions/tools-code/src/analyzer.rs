//! Code analysis utilities.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Code element type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Import,
    Variable,
    Constant,
    Type,
    Unknown,
}

/// A code element found during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: ElementType,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

/// Analysis result for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub path: String,
    pub language: String,
    pub elements: Vec<CodeElement>,
    pub imports: Vec<String>,
    pub line_count: usize,
}

/// Detect language from file extension.
pub fn detect_language(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?;
    let lang = match extension {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" => "javascript",
        "ts" | "mts" => "typescript",
        "jsx" => "javascript-react",
        "tsx" => "typescript-react",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" | "hpp" => "c-header",
        "rb" => "ruby",
        "php" => "php",
        "cs" => "csharp",
        "scala" => "scala",
        "clj" => "clojure",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "lua" => "lua",
        "sh" | "bash" => "shell",
        "sql" => "sql",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "md" | "markdown" => "markdown",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Simple pattern-based code analyzer.
pub struct PatternAnalyzer;

impl PatternAnalyzer {
    /// Analyze Rust code.
    pub fn analyze_rust(content: &str) -> Vec<CodeElement> {
        let mut elements = Vec::new();

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Function/method detection
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                if let Some(name) = Self::extract_rust_fn_name(trimmed) {
                    elements.push(CodeElement {
                        element_type: ElementType::Function,
                        name,
                        start_line: i + 1,
                        end_line: i + 1, // Simplified
                        signature: Some(trimmed.to_string()),
                        doc_comment: None,
                    });
                }
            }
            // Struct detection
            else if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                if let Some(name) = Self::extract_name_after(trimmed, "struct ") {
                    elements.push(CodeElement {
                        element_type: ElementType::Struct,
                        name,
                        start_line: i + 1,
                        end_line: i + 1,
                        signature: Some(trimmed.to_string()),
                        doc_comment: None,
                    });
                }
            }
            // Enum detection
            else if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                if let Some(name) = Self::extract_name_after(trimmed, "enum ") {
                    elements.push(CodeElement {
                        element_type: ElementType::Enum,
                        name,
                        start_line: i + 1,
                        end_line: i + 1,
                        signature: Some(trimmed.to_string()),
                        doc_comment: None,
                    });
                }
            }
            // Trait detection
            else if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                if let Some(name) = Self::extract_name_after(trimmed, "trait ") {
                    elements.push(CodeElement {
                        element_type: ElementType::Trait,
                        name,
                        start_line: i + 1,
                        end_line: i + 1,
                        signature: Some(trimmed.to_string()),
                        doc_comment: None,
                    });
                }
            }
            // Module detection
            else if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
                if let Some(name) = Self::extract_name_after(trimmed, "mod ") {
                    elements.push(CodeElement {
                        element_type: ElementType::Module,
                        name,
                        start_line: i + 1,
                        end_line: i + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
        }

        elements
    }

    fn extract_rust_fn_name(line: &str) -> Option<String> {
        let mut line = line;
        // Strip pub if present
        if let Some(rest) = line.strip_prefix("pub ") {
            line = rest.trim_start();
        }
        // Strip async if present
        if let Some(rest) = line.strip_prefix("async ") {
            line = rest.trim_start();
        }
        // Must start with fn
        let line = line.strip_prefix("fn ")?;
        let name_end = line.find(['(', '<'])?;
        Some(line[..name_end].to_string())
    }

    fn extract_name_after(line: &str, keyword: &str) -> Option<String> {
        let start = line.find(keyword)? + keyword.len();
        let rest = &line[start..];
        let end = rest.find(['{', '<', '(', ' ', ';'])?;
        Some(rest[..end].to_string())
    }
}

#[cfg(test)]
#[path = "analyzer_tests.rs"]
mod tests;
