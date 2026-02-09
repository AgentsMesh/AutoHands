//! Skill errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Skill loading failed: {0}")]
    LoadingFailed(String),

    #[error("Invalid skill definition: {0}")]
    InvalidDefinition(String),

    #[error("Skill parsing error: {0}")]
    ParsingError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = SkillError::NotFound("coding-skill".to_string());
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("coding-skill"));
    }

    #[test]
    fn test_loading_failed_error() {
        let err = SkillError::LoadingFailed("file not found".to_string());
        let display = err.to_string();
        assert!(display.contains("loading failed"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_invalid_definition_error() {
        let err = SkillError::InvalidDefinition("missing name field".to_string());
        let display = err.to_string();
        assert!(display.contains("Invalid skill definition"));
        assert!(display.contains("missing name field"));
    }

    #[test]
    fn test_parsing_error() {
        let err = SkillError::ParsingError("unexpected token".to_string());
        let display = err.to_string();
        assert!(display.contains("parsing error"));
        assert!(display.contains("unexpected token"));
    }

    #[test]
    fn test_error_debug() {
        let err = SkillError::NotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors: Vec<SkillError> = vec![
            SkillError::NotFound("a".to_string()),
            SkillError::LoadingFailed("b".to_string()),
            SkillError::InvalidDefinition("c".to_string()),
            SkillError::ParsingError("d".to_string()),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty());
        }
    }
}
