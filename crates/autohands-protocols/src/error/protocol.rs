//! Top-level protocol error type.

use thiserror::Error;

use super::{ChannelError, ExtensionError, MemoryError, ProviderError, ToolError};

/// Top-level protocol error type.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Extension error: {0}")]
    Extension(#[from] ExtensionError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_error_from() {
        let ext_err = ExtensionError::NotFound("test".to_string());
        let err = ProtocolError::from(ext_err);
        assert!(err.to_string().contains("Extension error"));
    }

    #[test]
    fn test_tool_error_from() {
        let tool_err = ToolError::NotFound("test".to_string());
        let err = ProtocolError::from(tool_err);
        assert!(err.to_string().contains("Tool error"));
    }

    #[test]
    fn test_provider_error_from() {
        let prov_err = ProviderError::NotFound("test".to_string());
        let err = ProtocolError::from(prov_err);
        assert!(err.to_string().contains("Provider error"));
    }

    #[test]
    fn test_channel_error_from() {
        let chan_err = ChannelError::Disconnected;
        let err = ProtocolError::from(chan_err);
        assert!(err.to_string().contains("Channel error"));
    }

    #[test]
    fn test_memory_error_from() {
        let mem_err = MemoryError::NotFound("test".to_string());
        let err = ProtocolError::from(mem_err);
        assert!(err.to_string().contains("Memory error"));
    }

    #[test]
    fn test_validation_error() {
        let err = ProtocolError::Validation("field required".to_string());
        assert!(err.to_string().contains("Validation error"));
        assert!(err.to_string().contains("field required"));
    }

    #[test]
    fn test_serialization_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = ProtocolError::from(json_err);
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_debug() {
        let err = ProtocolError::Validation("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Validation"));
    }
}
