//! Markdown memory errors.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur in Markdown memory operations.
#[derive(Debug, Error)]
pub enum MarkdownMemoryError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse YAML front matter.
    #[error("Failed to parse YAML front matter: {0}")]
    YamlParse(String),

    /// Failed to serialize YAML front matter.
    #[error("Failed to serialize YAML front matter: {0}")]
    YamlSerialize(String),

    /// Invalid front matter format.
    #[error("Invalid front matter format in file: {path}")]
    InvalidFrontMatter { path: PathBuf },

    /// Memory not found.
    #[error("Memory not found: {0}")]
    NotFound(String),

    /// Storage path not set.
    #[error("Storage path not set")]
    StoragePathNotSet,

    /// Failed to create storage directory.
    #[error("Failed to create storage directory at {path}: {reason}")]
    CreateDirFailed { path: PathBuf, reason: String },

    /// Invalid memory ID.
    #[error("Invalid memory ID: {0}")]
    InvalidId(String),
}

impl From<MarkdownMemoryError> for autohands_protocols::error::MemoryError {
    fn from(err: MarkdownMemoryError) -> Self {
        match err {
            MarkdownMemoryError::NotFound(id) => autohands_protocols::error::MemoryError::NotFound(id),
            MarkdownMemoryError::StoragePathNotSet => {
                autohands_protocols::error::MemoryError::ConnectionError("Storage path not set".to_string())
            }
            _ => autohands_protocols::error::MemoryError::QueryError(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_display() {
        let err = MarkdownMemoryError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_yaml_parse_error() {
        let err = MarkdownMemoryError::YamlParse("invalid yaml".to_string());
        assert!(err.to_string().contains("YAML front matter"));
    }

    #[test]
    fn test_not_found_error() {
        let err = MarkdownMemoryError::NotFound("mem_123".to_string());
        assert!(err.to_string().contains("mem_123"));
    }

    #[test]
    fn test_conversion_to_memory_error() {
        let err = MarkdownMemoryError::NotFound("test".to_string());
        let memory_err: autohands_protocols::error::MemoryError = err.into();
        assert!(matches!(memory_err, autohands_protocols::error::MemoryError::NotFound(_)));
    }
}
