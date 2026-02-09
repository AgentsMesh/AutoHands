//! Memory backend errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memory entry not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = MemoryError::NotFound("entry-123".to_string());
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("entry-123"));
    }

    #[test]
    fn test_storage_error() {
        let err = MemoryError::StorageError("disk full".to_string());
        let display = err.to_string();
        assert!(display.contains("Storage error"));
        assert!(display.contains("disk full"));
    }

    #[test]
    fn test_query_error() {
        let err = MemoryError::QueryError("invalid syntax".to_string());
        let display = err.to_string();
        assert!(display.contains("Query error"));
        assert!(display.contains("invalid syntax"));
    }

    #[test]
    fn test_serialization_error() {
        let err = MemoryError::SerializationError("invalid JSON".to_string());
        let display = err.to_string();
        assert!(display.contains("Serialization error"));
        assert!(display.contains("invalid JSON"));
    }

    #[test]
    fn test_connection_error() {
        let err = MemoryError::ConnectionError("connection refused".to_string());
        let display = err.to_string();
        assert!(display.contains("Connection error"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_error_debug() {
        let err = MemoryError::NotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors: Vec<MemoryError> = vec![
            MemoryError::NotFound("a".to_string()),
            MemoryError::StorageError("b".to_string()),
            MemoryError::QueryError("c".to_string()),
            MemoryError::SerializationError("d".to_string()),
            MemoryError::ConnectionError("e".to_string()),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty());
        }
    }
}
