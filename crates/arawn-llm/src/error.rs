use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    Api(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("rate limited: {message}")]
    RateLimited {
        message: String,
        /// Server-suggested wait parsed from a `Retry-After` header, if the
        /// provider sent one. The retry layer honors this instead of its own
        /// exponential backoff.
        retry_after: Option<std::time::Duration>,
    },

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
            LlmError::RateLimited { .. } => true,
            LlmError::ServerError(_) => true,
            LlmError::Request(e) => {
                // Timeouts are always worth a retry.
                if e.is_timeout() {
                    return true;
                }
                // DNS resolution and TLS failures are effectively permanent for
                // a given config (a typo'd provider URL, an expired cert) —
                // retrying just burns 30+ seconds of backoff before the user
                // sees the real problem. Bail out fast.
                if is_permanent_transport_error(e) {
                    return false;
                }
                // Genuine connection-level transients (resets, refused) are
                // retryable. `is_request()` (request *construction*) is NOT —
                // those are permanent client/config bugs, not transient.
                e.is_connect()
            }
            // Certain API errors are transient (malformed LLM output that may succeed on retry).
            // - `tool_use_failed`: Anthropic's signal for malformed tool-call output.
            // - `overloaded`: Anthropic's transient capacity signal.
            // - `failed to call a function` / `failed to parse tool call arguments`:
            //   Groq's strict server-side tool-call validation rejects native-format
            //   tool calls (Llama XML, Qwen <tool_call>, gpt-oss harmony tokens).
            //   Resampling usually wins because the model emits different output.
            //   Today these come back as 5xx (ServerError above); the message-pattern
            //   catch is belt-and-suspenders for any code path where Groq surfaces
            //   them as 4xx instead. See ARAWN-T-0411 / muninn writeup.
            LlmError::Api(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("tool_use_failed")
                    || lower.contains("overloaded")
                    || lower.contains("failed to call a function")
                    || lower.contains("failed to parse tool call arguments")
            }
            // JSON parse failures from LLM output are transient — a retry may produce valid JSON
            LlmError::Json(_) => true,
            LlmError::Stream(_) => false,
            LlmError::Auth(_) => false,
            LlmError::ModelNotFound(_) => false,
            LlmError::Config(_) => false,
        }
    }

    /// Create from an HTTP status code + body.
    pub fn from_status(status: u16, body: String) -> Self {
        Self::from_status_with_retry_after(status, body, None)
    }

    /// Like [`from_status`], but threads a `Retry-After` duration (extracted
    /// from the response headers by the provider client) onto a 429.
    pub fn from_status_with_retry_after(
        status: u16,
        body: String,
        retry_after: Option<std::time::Duration>,
    ) -> Self {
        // Try to extract a clean error message from JSON response bodies
        let message = extract_api_message(&body).unwrap_or(body);

        match status {
            401 => LlmError::Auth(format!("HTTP 401: {message}")),
            403 => LlmError::Auth(format!("HTTP 403: {message}")),
            404 => LlmError::ModelNotFound(format!("HTTP 404: {message}")),
            // 408 Request Timeout is transient — the request never completed,
            // a retry can succeed. Classed as a server error so the retry
            // layer picks it up.
            408 => LlmError::ServerError(format!("HTTP 408 (request timeout): {message}")),
            429 => LlmError::RateLimited {
                message: format!("HTTP 429: {message}"),
                retry_after,
            },
            500..=599 => LlmError::ServerError(format!("HTTP {status}: {message}")),
            _ => LlmError::Api(format!("HTTP {status}: {message}")),
        }
    }

    /// The server-suggested retry delay, if this is a rate-limit error that
    /// carried a `Retry-After`.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            LlmError::RateLimited { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Return a user-facing error message with actionable guidance.
    pub fn user_message(&self) -> String {
        match self {
            LlmError::Auth(_) => {
                // Provider-agnostic: the exact env var differs per provider
                // and the startup fail-fast path (build_llm_client) names it
                // precisely. At runtime we only know the request was rejected.
                "Authentication failed — check that your API key is valid and set \
                 (via `api_key` or the `api_key_env` for this provider in arawn.toml)."
                    .to_string()
            }
            LlmError::ModelNotFound(msg) => {
                format!(
                    "Model not found — the requested model may not be available on this provider. \
                     Check the model name in arawn.toml. ({msg})"
                )
            }
            LlmError::RateLimited { retry_after, .. } => match retry_after {
                Some(d) => format!(
                    "Rate limited by the API provider. Arawn will retry automatically \
                     after the server-suggested {}s. If this persists, check your plan limits.",
                    d.as_secs()
                ),
                None => "Rate limited by the API provider. Arawn will retry automatically \
                         with exponential backoff. If this persists, check your plan limits."
                    .to_string(),
            },
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

/// Heuristic: is this reqwest error a *permanent* transport failure (DNS
/// resolution or TLS), as opposed to a transient connection problem?
///
/// reqwest lumps DNS, TLS, and TCP failures under `is_connect()`, so we have
/// to sniff the error's source chain to tell a typo'd host / bad certificate
/// (permanent) from a connection reset (transient, worth retrying).
fn is_permanent_transport_error(e: &reqwest::Error) -> bool {
    use std::error::Error;
    let mut src: Option<&dyn Error> = Some(e);
    while let Some(err) = src {
        let msg = err.to_string().to_lowercase();
        if msg.contains("dns")
            || msg.contains("failed to lookup address")
            || msg.contains("name or service not known")
            || msg.contains("nodename nor servname")
            || msg.contains("no such host")
            || msg.contains("certificate")
            || msg.contains("tls")
            || msg.contains("ssl")
            || msg.contains("handshake")
        {
            return true;
        }
        src = err.source();
    }
    false
}

/// Maximum length of `failed_generation` content to include in error
/// messages. Beyond this we truncate — native-format tool-call dumps
/// can be several KB and blowing up the log line buys nothing.
const FAILED_GENERATION_TRUNCATE_BYTES: usize = 2048;

/// Try to extract a clean message from a JSON error body.
///
/// Groq/OpenAI format: `{"error": {"message": "...", "type": "..."}}`.
///
/// When Groq's strict server-side tool-call validator rejects a native-
/// format tool call (Llama XML, Qwen `<tool_call>`, gpt-oss harmony
/// tokens) it returns an additional `error.failed_generation` field
/// carrying the raw model output. We surface it here so the operator
/// sees what the model actually emitted instead of just the generic
/// "Please adjust your prompt." — without it, diagnosis is guesswork.
/// See ARAWN-T-0411 for context.
fn extract_api_message(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let err = parsed.get("error")?;
    let message = err.get("message").and_then(|m| m.as_str())?.to_string();
    let failed_gen = err
        .get("failed_generation")
        .and_then(|m| m.as_str())
        .filter(|s| !s.is_empty());
    match failed_gen {
        None => Some(message),
        Some(fg) => {
            let truncated = if fg.len() > FAILED_GENERATION_TRUNCATE_BYTES {
                // Truncate on a UTF-8 char boundary just below the limit so
                // we never split a multi-byte sequence.
                let mut cut = FAILED_GENERATION_TRUNCATE_BYTES;
                while cut > 0 && !fg.is_char_boundary(cut) {
                    cut -= 1;
                }
                format!("{}…[truncated]", &fg[..cut])
            } else {
                fg.to_string()
            };
            Some(format!("{message} | failed_generation: {truncated}"))
        }
    }
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
    fn auth_user_message_is_provider_agnostic() {
        // Regression: the runtime auth message must NOT hardcode a single
        // provider's env var — it's shown regardless of which provider failed.
        let msg = LlmError::Auth("HTTP 401: bad key".into()).user_message();
        assert!(
            !msg.contains("GROQ_API_KEY"),
            "must not hardcode GROQ: {msg}"
        );
        assert!(msg.to_lowercase().contains("api key"));
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
        assert!(matches!(err, LlmError::RateLimited { .. }));
        assert!(err.is_retryable());
        assert!(err.user_message().contains("Rate limited"));
    }

    #[test]
    fn from_status_408_is_retryable_server_error() {
        let err = LlmError::from_status(408, "request timeout".into());
        assert!(matches!(err, LlmError::ServerError(_)));
        assert!(err.is_retryable(), "408 should be retried");
    }

    #[test]
    fn rate_limited_carries_retry_after() {
        let err = LlmError::from_status_with_retry_after(
            429,
            "slow down".into(),
            Some(std::time::Duration::from_secs(12)),
        );
        assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(12)));
        assert!(err.user_message().contains("12s"));
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

    // ─── ARAWN-T-0411 — Groq failed_generation surfacing ─────────────────

    #[test]
    fn extract_message_includes_failed_generation_when_present() {
        let body = r#"{"error":{"message":"Failed to call a function. Please adjust your prompt.","type":"backend_error","failed_generation":"<function=foo>{\"x\":1}</function>"}}"#;
        let msg = extract_api_message(body).expect("should parse");
        assert!(
            msg.contains("Failed to call a function"),
            "expected original message preserved, got: {msg}"
        );
        assert!(
            msg.contains("failed_generation: <function=foo>"),
            "expected failed_generation appended, got: {msg}"
        );
    }

    #[test]
    fn extract_message_truncates_long_failed_generation() {
        // 3KB string — should truncate to 2KB + "…[truncated]"
        let raw = "x".repeat(3072);
        let body = format!(
            r#"{{"error":{{"message":"m","type":"backend_error","failed_generation":"{raw}"}}}}"#
        );
        let msg = extract_api_message(&body).expect("should parse");
        assert!(
            msg.contains("…[truncated]"),
            "expected truncation suffix, got message of len {} ending: {}",
            msg.len(),
            &msg[msg.len().saturating_sub(40)..]
        );
        assert!(
            msg.len() < 3072,
            "expected truncated length, got {}",
            msg.len()
        );
    }

    #[test]
    fn extract_message_handles_empty_failed_generation() {
        // Empty string for failed_generation should not produce a hanging
        // "| failed_generation: " suffix.
        let body = r#"{"error":{"message":"some error","failed_generation":""}}"#;
        let msg = extract_api_message(body).expect("should parse");
        assert_eq!(msg, "some error");
    }

    #[test]
    fn extract_message_without_failed_generation_preserves_old_behavior() {
        let body = r#"{"error":{"message":"plain old message"}}"#;
        let msg = extract_api_message(body).expect("should parse");
        assert_eq!(msg, "plain old message");
    }

    // ─── ARAWN-T-0411 — Groq retryable patterns ──────────────────────────

    #[test]
    fn api_error_failed_to_call_a_function_is_retryable() {
        let err =
            LlmError::Api("HTTP 400: Failed to call a function. Please adjust your prompt.".into());
        assert!(err.is_retryable());
    }

    #[test]
    fn api_error_failed_to_parse_tool_call_arguments_is_retryable() {
        let err = LlmError::Api("HTTP 400: Failed to parse tool call arguments".into());
        assert!(err.is_retryable());
    }

    #[test]
    fn api_error_invalid_model_is_not_retryable() {
        // Anti-regression: random 4xx still not retryable.
        let err = LlmError::Api("HTTP 400: invalid model".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn api_error_failed_to_call_case_insensitive() {
        // Defensive: provider might shift capitalization.
        let err = LlmError::Api("HTTP 502: FAILED TO CALL A FUNCTION".into());
        assert!(err.is_retryable());
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
