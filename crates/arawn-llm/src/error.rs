use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    Api(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("server error: {0}")]
    ServerError(String),

    #[error("stream error: {0}")]
    Stream(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl LlmError {
    /// Returns true if this error is transient and the request should be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            LlmError::RateLimited(_) => true,
            LlmError::ServerError(_) => true,
            LlmError::Request(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            LlmError::Stream(_) => false,
            LlmError::Api(_) => false,
            LlmError::Auth(_) => false,
            LlmError::ModelNotFound(_) => false,
            LlmError::Config(_) => false,
            LlmError::Json(_) => false,
        }
    }

    /// Create from an HTTP status code + body.
    pub fn from_status(status: u16, body: String) -> Self {
        // Try to extract a clean error message from JSON response bodies
        let message = extract_api_message(&body).unwrap_or(body);

        match status {
            401 => LlmError::Auth(format!("HTTP 401: {message}")),
            403 => LlmError::Auth(format!("HTTP 403: {message}")),
            404 => LlmError::ModelNotFound(format!("HTTP 404: {message}")),
            429 => LlmError::RateLimited(format!("HTTP 429: {message}")),
            500..=599 => LlmError::ServerError(format!("HTTP {status}: {message}")),
            _ => LlmError::Api(format!("HTTP {status}: {message}")),
        }
    }

    /// Return a user-facing error message with actionable guidance.
    pub fn user_message(&self) -> String {
        match self {
            LlmError::Auth(_) => {
                "Authentication failed — check that your API key is set correctly \
                 (GROQ_API_KEY environment variable)."
                    .to_string()
            }
            LlmError::ModelNotFound(msg) => {
                format!(
                    "Model not found — the requested model may not be available on this provider. \
                     Check the model name in arawn.toml. ({msg})"
                )
            }
            LlmError::RateLimited(_) => {
                "Rate limited by the API provider. Arawn will retry automatically \
                 with exponential backoff. If this persists, check your plan limits."
                    .to_string()
            }
            LlmError::ServerError(_) => {
                "The API provider returned a server error. This is usually temporary — \
                 Arawn will retry automatically."
                    .to_string()
            }
            LlmError::Config(msg) => {
                format!("Configuration error: {msg}")
            }
            LlmError::Request(e) => {
                if e.is_timeout() {
                    "Request timed out — the API provider may be slow or unreachable. \
                     Check your network connection."
                        .to_string()
                } else if e.is_connect() {
                    "Could not connect to the API provider. Check your network connection \
                     and that the provider URL is correct."
                        .to_string()
                } else {
                    format!("Network error: {e}")
                }
            }
            LlmError::Stream(msg) => {
                format!("Streaming error — the response was interrupted: {msg}")
            }
            LlmError::Json(e) => {
                format!(
                    "Failed to parse API response — this may indicate an incompatible \
                     API provider or model. ({e})"
                )
            }
            LlmError::Api(msg) => {
                format!("API error: {msg}")
            }
        }
    }
}

/// Try to extract a clean message from a JSON error body.
/// Groq/OpenAI format: {"error": {"message": "...", "type": "..."}}
fn extract_api_message(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_status_401_is_auth() {
        let err = LlmError::from_status(401, r#"{"error":{"message":"Invalid API key"}}"#.into());
        assert!(matches!(err, LlmError::Auth(_)));
        assert!(!err.is_retryable());
        assert!(err.user_message().contains("API key"));
    }

    #[test]
    fn from_status_403_is_auth() {
        let err = LlmError::from_status(403, "forbidden".into());
        assert!(matches!(err, LlmError::Auth(_)));
    }

    #[test]
    fn from_status_404_is_model_not_found() {
        let err = LlmError::from_status(
            404,
            r#"{"error":{"message":"model 'foo' not found"}}"#.into(),
        );
        assert!(matches!(err, LlmError::ModelNotFound(_)));
        assert!(err.user_message().contains("model"));
        assert!(err.user_message().contains("arawn.toml"));
    }

    #[test]
    fn from_status_429_is_rate_limited() {
        let err = LlmError::from_status(429, "too many requests".into());
        assert!(matches!(err, LlmError::RateLimited(_)));
        assert!(err.is_retryable());
        assert!(err.user_message().contains("Rate limited"));
    }

    #[test]
    fn from_status_500_is_server_error() {
        let err = LlmError::from_status(500, "internal server error".into());
        assert!(matches!(err, LlmError::ServerError(_)));
        assert!(err.is_retryable());
        assert!(err.user_message().contains("server error"));
    }

    #[test]
    fn from_status_400_is_api_error() {
        let err = LlmError::from_status(400, "bad request".into());
        assert!(matches!(err, LlmError::Api(_)));
        assert!(!err.is_retryable());
    }

    #[test]
    fn extract_message_from_json_body() {
        let body = r#"{"error":{"message":"Model not available","type":"invalid_request"}}"#;
        let msg = extract_api_message(body).unwrap();
        assert_eq!(msg, "Model not available");
    }

    #[test]
    fn extract_message_from_plain_text_returns_none() {
        assert!(extract_api_message("just a string").is_none());
    }

    #[test]
    fn config_error_user_message() {
        let err = LlmError::Config("missing API key".into());
        assert!(err.user_message().contains("missing API key"));
    }

    #[test]
    fn stream_error_user_message() {
        let err = LlmError::Stream("connection reset".into());
        assert!(err.user_message().contains("interrupted"));
    }
}
