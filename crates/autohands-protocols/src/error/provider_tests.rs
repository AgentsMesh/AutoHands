use super::*;

#[test]
fn test_provider_error_not_found() {
    let err = ProviderError::NotFound("test_provider".to_string());
    assert!(err.to_string().contains("Provider not found"));
}

#[test]
fn test_provider_error_model_not_found() {
    let err = ProviderError::ModelNotFound("gpt-5".to_string());
    assert!(err.to_string().contains("Model not found"));
}

#[test]
fn test_provider_error_api_error() {
    let err = ProviderError::ApiError {
        status: 500,
        message: "Internal Server Error".to_string(),
    };
    assert!(err.to_string().contains("500"));
    assert!(err.to_string().contains("Internal Server Error"));
}

#[test]
fn test_provider_error_rate_limited() {
    let err = ProviderError::RateLimited {
        retry_after_seconds: 60,
    };
    assert!(err.to_string().contains("Rate limited"));
    assert!(err.to_string().contains("60"));
}

#[test]
fn test_provider_error_auth_failed() {
    let err = ProviderError::AuthenticationFailed("Invalid API key".to_string());
    assert!(err.to_string().contains("Authentication failed"));
}

#[test]
fn test_provider_error_invalid_request() {
    let err = ProviderError::InvalidRequest("Missing model".to_string());
    assert!(err.to_string().contains("Invalid request"));
}

#[test]
fn test_provider_error_context_length() {
    let err = ProviderError::ContextLengthExceeded {
        used: 150000,
        max: 128000,
    };
    assert!(err.to_string().contains("150000"));
    assert!(err.to_string().contains("128000"));
}

#[test]
fn test_provider_error_content_filtered() {
    let err = ProviderError::ContentFiltered("Inappropriate content".to_string());
    assert!(err.to_string().contains("Content filtered"));
}

#[test]
fn test_provider_error_network() {
    let err = ProviderError::Network("Connection refused".to_string());
    assert!(err.to_string().contains("Network error"));
}

#[test]
fn test_provider_error_stream() {
    let err = ProviderError::StreamError("Stream closed unexpectedly".to_string());
    assert!(err.to_string().contains("Stream error"));
}

#[test]
fn test_provider_error_timeout() {
    let err = ProviderError::Timeout(30);
    assert!(err.to_string().contains("Timeout"));
    assert!(err.to_string().contains("30"));
}

#[test]
fn test_provider_error_debug() {
    let err = ProviderError::NotFound("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("NotFound"));
}

#[test]
fn test_from_api_response_auth_failed() {
    let err = ProviderError::from_api_response(401, "Invalid API key".to_string());
    assert!(matches!(err, ProviderError::AuthenticationFailed(_)));
}

#[test]
fn test_from_api_response_rate_limited() {
    let err = ProviderError::from_api_response(429, "Rate limit exceeded".to_string());
    assert!(matches!(err, ProviderError::RateLimited { .. }));
}

#[test]
fn test_from_api_response_token_exceed() {
    let err = ProviderError::from_api_response(
        400,
        "Total tokens exceed the maximum limit".to_string(),
    );
    assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
}

#[test]
fn test_from_api_response_context_length() {
    let err = ProviderError::from_api_response(
        400,
        "This model's maximum context length is 128000".to_string(),
    );
    assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
}

#[test]
fn test_from_api_response_too_many_tokens() {
    let err = ProviderError::from_api_response(
        400,
        "Request has too many tokens".to_string(),
    );
    assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
}

#[test]
fn test_from_api_response_content_filtered() {
    let err = ProviderError::from_api_response(
        400,
        "Content filter triggered".to_string(),
    );
    assert!(matches!(err, ProviderError::ContentFiltered(_)));
}

#[test]
fn test_from_api_response_safety_blocked() {
    let err = ProviderError::from_api_response(
        400,
        "Response blocked by safety settings".to_string(),
    );
    assert!(matches!(err, ProviderError::ContentFiltered(_)));
}

#[test]
fn test_from_api_response_generic_error() {
    let err = ProviderError::from_api_response(
        500,
        "Internal Server Error".to_string(),
    );
    assert!(matches!(err, ProviderError::ApiError { status: 500, .. }));
}

#[test]
fn test_is_retryable() {
    assert!(ProviderError::RateLimited { retry_after_seconds: 5 }.is_retryable());
    assert!(ProviderError::ContextLengthExceeded { used: 0, max: 0 }.is_retryable());
    assert!(ProviderError::Network("err".to_string()).is_retryable());
    assert!(ProviderError::Timeout(30).is_retryable());
    assert!(!ProviderError::AuthenticationFailed("err".to_string()).is_retryable());
    assert!(!ProviderError::ApiError { status: 500, message: "err".to_string() }.is_retryable());
}

#[test]
fn test_is_context_length_error() {
    assert!(ProviderError::ContextLengthExceeded { used: 0, max: 0 }.is_context_length_error());
    assert!(!ProviderError::RateLimited { retry_after_seconds: 0 }.is_context_length_error());
    assert!(!ProviderError::Network("err".to_string()).is_context_length_error());
}
