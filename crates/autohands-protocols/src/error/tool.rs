//! Tool execution errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Parameter validation failed: {0}")]
    ValidationFailed(String),

    #[error("Tool execution timed out after {0} seconds")]
    Timeout(u64),

    #[error("Tool execution was cancelled")]
    Cancelled,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_error_not_found() {
        let err = ToolError::NotFound("test_tool".to_string());
        assert!(err.to_string().contains("Tool not found"));
        assert!(err.to_string().contains("test_tool"));
    }

    #[test]
    fn test_tool_error_execution_failed() {
        let err = ToolError::ExecutionFailed("Something went wrong".to_string());
        assert!(err.to_string().contains("execution failed"));
    }

    #[test]
    fn test_tool_error_invalid_parameters() {
        let err = ToolError::InvalidParameters("missing field".to_string());
        assert!(err.to_string().contains("Invalid parameters"));
    }

    #[test]
    fn test_tool_error_validation_failed() {
        let err = ToolError::ValidationFailed("type mismatch".to_string());
        assert!(err.to_string().contains("validation failed"));
    }

    #[test]
    fn test_tool_error_timeout() {
        let err = ToolError::Timeout(30);
        assert!(err.to_string().contains("timed out"));
        assert!(err.to_string().contains("30"));
    }

    #[test]
    fn test_tool_error_cancelled() {
        let err = ToolError::Cancelled;
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn test_tool_error_permission_denied() {
        let err = ToolError::PermissionDenied("read /etc/passwd".to_string());
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_tool_error_resource_not_found() {
        let err = ToolError::ResourceNotFound("/tmp/missing".to_string());
        assert!(err.to_string().contains("Resource not found"));
    }

    #[test]
    fn test_tool_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ToolError = io_err.into();
        assert!(err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_tool_error_debug() {
        let err = ToolError::NotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }
}
