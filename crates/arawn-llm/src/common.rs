//! Shared utilities for LLM backend implementations.
//!
//! Extracted from duplicate code between `openai.rs` and `anthropic.rs`.
//! Contains HTTP client construction, error response parsing, retry-after
//! extraction, and stop reason mapping.

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::error::LlmError;
use crate::types::StopReason;

// ─────────────────────────────────────────────────────────────────────────────
// HTTP Client Construction
// ─────────────────────────────────────────────────────────────────────────────

/// Build a reqwest HTTP client with the given timeout.
///
/// Both OpenAI and Anthropic backends use identical client construction.
pub fn build_http_client(timeout: Duration) -> Result<Client, LlmError> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| LlmError::Internal(format!("Failed to create HTTP client: {}", e)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Error Response
// ─────────────────────────────────────────────────────────────────────────────

/// Generic provider error response shape.
///
/// Both OpenAI and Anthropic return `{"error": {"message": "..."}}`.
/// The outer wrapper struct differs in name but has the same shape.
#[derive(Debug, Deserialize)]
pub struct ProviderErrorResponse {
    pub error: ProviderErrorDetail,
}

/// Error detail with a human-readable message.
#[derive(Debug, Deserialize)]
pub struct ProviderErrorDetail {
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Retry-After Extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the `Retry-After` header value from response headers.
///
/// Returns the header value as a string, or None if not present.
pub fn extract_retry_after(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Status Mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Map an HTTP error status + parsed error body to an `LlmError`.
///
/// Shared logic for 401 (auth), 429 (rate limit), 5xx (backend), and other errors.
/// The `groq_style` parameter enables Groq's inline retry timing in the message body.
pub fn map_error_response(
    status: u16,
    error_message: &str,
    retry_after: Option<&str>,
    groq_style_retry: bool,
) -> LlmError {
    match status {
        401 => LlmError::Auth(format!("Authentication failed: {}", error_message)),
        429 => {
            if groq_style_retry
                && (error_message.contains("try again in")
                    || error_message.contains("Try again in"))
            {
                let info = crate::error::RateLimitInfo::parse_groq(error_message);
                LlmError::RateLimit(info)
            } else {
                let info = crate::error::RateLimitInfo::parse_openai(error_message, retry_after);
                LlmError::RateLimit(info)
            }
        }
        500..=599 => LlmError::Backend(format!("Server error: {}", error_message)),
        _ => LlmError::Backend(error_message.to_string()),
    }
}

/// Map an HTTP error response when the body couldn't be parsed as a provider error.
pub fn map_raw_error(status: reqwest::StatusCode, body: &str) -> LlmError {
    LlmError::Backend(format!("HTTP {}: {}", status, body))
}

// ─────────────────────────────────────────────────────────────────────────────
// Stop Reason Mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Map a provider-specific stop reason string to a `StopReason`.
///
/// OpenAI uses: "stop", "tool_calls", "length"
/// Anthropic uses: "end_turn", "tool_use", "max_tokens", "stop_sequence"
pub fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        // OpenAI conventions
        "stop" => StopReason::EndTurn,
        "tool_calls" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        // Anthropic conventions
        "end_turn" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        // Default
        _ => StopReason::EndTurn,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_http_client() {
        let client = build_http_client(Duration::from_secs(30));
        assert!(client.is_ok());
    }

    #[test]
    fn test_provider_error_response_parse() {
        let json = r#"{"error": {"message": "Invalid API key"}}"#;
        let parsed: ProviderErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.error.message, "Invalid API key");
    }

    #[test]
    fn test_map_stop_reason_openai() {
        assert!(matches!(map_stop_reason("stop"), StopReason::EndTurn));
        assert!(matches!(map_stop_reason("tool_calls"), StopReason::ToolUse));
        assert!(matches!(map_stop_reason("length"), StopReason::MaxTokens));
    }

    #[test]
    fn test_map_stop_reason_anthropic() {
        assert!(matches!(map_stop_reason("end_turn"), StopReason::EndTurn));
        assert!(matches!(map_stop_reason("tool_use"), StopReason::ToolUse));
        assert!(matches!(
            map_stop_reason("max_tokens"),
            StopReason::MaxTokens
        ));
        assert!(matches!(
            map_stop_reason("stop_sequence"),
            StopReason::StopSequence
        ));
    }

    #[test]
    fn test_map_stop_reason_unknown_defaults_to_end_turn() {
        assert!(matches!(map_stop_reason("unknown"), StopReason::EndTurn));
        assert!(matches!(map_stop_reason(""), StopReason::EndTurn));
    }

    #[test]
    fn test_map_error_response_auth() {
        let err = map_error_response(401, "bad key", None, false);
        assert!(matches!(err, LlmError::Auth(_)));
    }

    #[test]
    fn test_map_error_response_rate_limit() {
        let err = map_error_response(429, "too many requests", Some("5"), false);
        assert!(matches!(err, LlmError::RateLimit(_)));
    }

    #[test]
    fn test_map_error_response_server_error() {
        let err = map_error_response(500, "internal", None, false);
        assert!(matches!(err, LlmError::Backend(_)));
        assert!(err.to_string().contains("Server error"));
    }

    #[test]
    fn test_extract_retry_after() {
        let mut headers = reqwest::header::HeaderMap::new();
        assert!(extract_retry_after(&headers).is_none());

        headers.insert("retry-after", "5".parse().unwrap());
        assert_eq!(extract_retry_after(&headers), Some("5".to_string()));
    }
}
