use orbit_providers::ApiError;
use reqwest::StatusCode;

#[test]
fn missing_credentials_display_includes_provider_and_env_vars() {
    let error = ApiError::missing_credentials("anthropic", &["ANTHROPIC_API_KEY"]);
    let display = format!("{error}");
    assert!(display.contains("anthropic"));
    assert!(display.contains("ANTHROPIC_API_KEY"));
}

#[test]
fn context_window_exceeded_display_includes_details() {
    let error = ApiError::ContextWindowExceeded {
        model: "claude-3".to_string(),
        estimated_input_tokens: 100_000,
        requested_output_tokens: 8_192,
        estimated_total_tokens: 108_192,
        context_window_tokens: 100_000,
    };
    let display = format!("{error}");
    assert!(display.contains("context_window_blocked"));
    assert!(display.contains("claude-3"));
}

#[test]
fn expired_oauth_token_display() {
    let error = ApiError::ExpiredOAuthToken;
    let display = format!("{error}");
    assert!(display.contains("OAuth"));
}

#[test]
fn auth_error_display() {
    let error = ApiError::Auth("invalid key".to_string());
    let display = format!("{error}");
    assert!(display.contains("invalid key"));
}

#[test]
fn api_error_display_includes_status_and_message() {
    let error = ApiError::Api {
        status: StatusCode::BAD_REQUEST,
        error_type: Some("invalid_request_error".to_string()),
        message: Some("bad request".to_string()),
        request_id: Some("req_123".to_string()),
        body: String::new(),
        retryable: false,
    };
    let display = format!("{error}");
    assert!(display.contains("400"));
    assert!(display.contains("bad request"));
    assert!(display.contains("req_123"));
}

#[test]
fn api_error_display_without_message_uses_body() {
    let error = ApiError::Api {
        status: StatusCode::TOO_MANY_REQUESTS,
        error_type: None,
        message: None,
        request_id: None,
        body: "rate limited".to_string(),
        retryable: true,
    };
    let display = format!("{error}");
    assert!(!display.is_empty());
}

#[test]
fn retries_exhausted_display_includes_attempts() {
    let error = ApiError::RetriesExhausted {
        attempts: 3,
        last_error: Box::new(ApiError::Auth("rate limited".to_string())),
    };
    let display = format!("{error}");
    assert!(display.contains('3'));
}

#[test]
fn is_retryable_api_with_flag() {
    let error = ApiError::Api {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        error_type: None,
        message: None,
        request_id: None,
        body: String::new(),
        retryable: true,
    };
    assert!(error.is_retryable());
}

#[test]
fn is_not_retryable_auth_errors() {
    let error = ApiError::Auth("bad key".to_string());
    assert!(!error.is_retryable());
}

#[test]
fn is_not_retryable_missing_credentials() {
    let error = ApiError::missing_credentials("anthropic", &["ANTHROPIC_API_KEY"]);
    assert!(!error.is_retryable());
}

#[test]
fn is_not_retryable_context_window() {
    let error = ApiError::ContextWindowExceeded {
        model: "claude-3".to_string(),
        estimated_input_tokens: 100_000,
        requested_output_tokens: 8_192,
        estimated_total_tokens: 108_192,
        context_window_tokens: 100_000,
    };
    assert!(!error.is_retryable());
}

#[test]
fn safe_failure_class_auth() {
    let error = ApiError::Auth("bad key".to_string());
    assert_eq!(error.safe_failure_class(), "provider_auth");
}

#[test]
fn safe_failure_class_missing_credentials() {
    let error = ApiError::missing_credentials("openai", &["OPENAI_API_KEY"]);
    assert_eq!(error.safe_failure_class(), "provider_auth");
}

#[test]
fn safe_failure_class_context_window() {
    let error = ApiError::ContextWindowExceeded {
        model: "claude-3".to_string(),
        estimated_input_tokens: 100_000,
        requested_output_tokens: 8_192,
        estimated_total_tokens: 108_192,
        context_window_tokens: 100_000,
    };
    assert_eq!(error.safe_failure_class(), "context_window");
}

#[test]
fn safe_failure_class_rate_limit() {
    let error = ApiError::Api {
        status: StatusCode::TOO_MANY_REQUESTS,
        error_type: Some("rate_limit".to_string()),
        message: None,
        request_id: None,
        body: "rate limited".to_string(),
        retryable: false,
    };
    assert_eq!(error.safe_failure_class(), "provider_rate_limit");
}

#[test]
fn safe_failure_class_api_401() {
    let error = ApiError::Api {
        status: StatusCode::UNAUTHORIZED,
        error_type: None,
        message: None,
        request_id: None,
        body: "unauthorized".to_string(),
        retryable: false,
    };
    assert_eq!(error.safe_failure_class(), "provider_auth");
}

#[test]
fn safe_failure_class_transport() {
    let error = ApiError::InvalidSseFrame("unexpected frame");
    assert_eq!(error.safe_failure_class(), "provider_transport");
}

#[test]
fn safe_failure_class_runtime_io() {
    let error = ApiError::Io(std::io::Error::other("disk error"));
    assert_eq!(error.safe_failure_class(), "runtime_io");
}

#[test]
fn request_id_propagation() {
    let inner = ApiError::Api {
        status: StatusCode::BAD_REQUEST,
        error_type: None,
        message: None,
        request_id: Some("req_abc".to_string()),
        body: String::new(),
        retryable: false,
    };
    assert_eq!(inner.request_id(), Some("req_abc"));

    let exhausted = ApiError::RetriesExhausted {
        attempts: 3,
        last_error: Box::new(inner),
    };
    assert_eq!(exhausted.request_id(), Some("req_abc"));
}

#[test]
fn is_context_window_failure_from_message() {
    let error = ApiError::Api {
        status: StatusCode::BAD_REQUEST,
        error_type: Some("invalid_request_error".to_string()),
        message: Some("This model's maximum context length is 200000 tokens".to_string()),
        request_id: None,
        body: String::new(),
        retryable: false,
    };
    assert!(error.is_context_window_failure());
}

#[test]
fn is_context_window_failure_from_body() {
    let error = ApiError::Api {
        status: StatusCode::BAD_REQUEST,
        error_type: None,
        message: None,
        request_id: None,
        body: "maximum context length exceeded".to_string(),
        retryable: false,
    };
    assert!(error.is_context_window_failure());
}

#[test]
fn not_context_window_for_other_errors() {
    let error = ApiError::Auth("bad key".to_string());
    assert!(!error.is_context_window_failure());
}

#[test]
fn invalid_sse_frame_display() {
    let error = ApiError::InvalidSseFrame("unexpected delimiter");
    let display = format!("{error}");
    assert!(display.contains("unexpected delimiter"));
}

#[test]
fn backoff_overflow_display() {
    let error = ApiError::BackoffOverflow {
        attempt: 5,
        base_delay: std::time::Duration::from_secs(1),
    };
    let display = format!("{error}");
    assert!(display.contains('5'));
}
