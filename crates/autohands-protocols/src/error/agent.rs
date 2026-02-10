//! Agent errors.

use thiserror::Error;

use super::ProviderError;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Max turns exceeded: {0}")]
    MaxTurnsExceeded(u32),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Agent was aborted")]
    Aborted,

    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_error_not_found() {
        let err = AgentError::NotFound("test_agent".to_string());
        assert!(err.to_string().contains("Agent not found"));
        assert!(err.to_string().contains("test_agent"));
    }

    #[test]
    fn test_agent_error_execution_failed() {
        let err = AgentError::ExecutionFailed("Something went wrong".to_string());
        assert!(err.to_string().contains("execution failed"));
    }

    #[test]
    fn test_agent_error_max_turns() {
        let err = AgentError::MaxTurnsExceeded(50);
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_agent_error_timeout() {
        let err = AgentError::Timeout(300);
        assert!(err.to_string().contains("300"));
        assert!(err.to_string().contains("seconds"));
    }

    #[test]
    fn test_agent_error_aborted() {
        let err = AgentError::Aborted;
        assert!(err.to_string().contains("aborted"));
    }

    #[test]
    fn test_agent_error_debug() {
        let err = AgentError::NotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }
}
