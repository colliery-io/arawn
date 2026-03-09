//! Domain error types.

use thiserror::Error;

/// Domain-level errors.
#[derive(Debug, Error)]
pub enum DomainError {
    /// Session not found.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Workstream not found.
    #[error("Workstream not found: {0}")]
    WorkstreamNotFound(String),

    /// Agent execution error.
    #[error("Agent error: {0}")]
    Agent(#[from] arawn_agent::AgentError),

    /// MCP server error.
    #[error("MCP error: {0}")]
    Mcp(String),

    /// Workstream error.
    #[error("Workstream error: {0}")]
    Workstream(#[from] arawn_workstream::WorkstreamError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for domain operations.
pub type Result<T> = std::result::Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_display() {
        let err = DomainError::SessionNotFound("sess-123".to_string());
        assert_eq!(err.to_string(), "Session not found: sess-123");
    }

    #[test]
    fn test_workstream_not_found_display() {
        let err = DomainError::WorkstreamNotFound("ws-456".to_string());
        assert_eq!(err.to_string(), "Workstream not found: ws-456");
    }

    #[test]
    fn test_mcp_error_display() {
        let err = DomainError::Mcp("connection refused".to_string());
        assert_eq!(err.to_string(), "MCP error: connection refused");
    }

    #[test]
    fn test_config_error_display() {
        let err = DomainError::Config("missing field".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing field");
    }

    #[test]
    fn test_internal_error_display() {
        let err = DomainError::Internal("unexpected state".to_string());
        assert_eq!(err.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_error_debug_impl() {
        let err = DomainError::SessionNotFound("x".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("SessionNotFound"));
    }

    #[test]
    fn test_empty_string_variants() {
        let err = DomainError::SessionNotFound(String::new());
        assert_eq!(err.to_string(), "Session not found: ");

        let err = DomainError::Mcp(String::new());
        assert_eq!(err.to_string(), "MCP error: ");
    }

    #[test]
    fn test_special_chars_in_error() {
        let err = DomainError::Internal("error: 'foo' & \"bar\" <baz>".to_string());
        assert_eq!(
            err.to_string(),
            "Internal error: error: 'foo' & \"bar\" <baz>"
        );
    }

    #[test]
    fn test_result_type_ok() {
        let ok: Result<i32> = Ok(42);
        assert!(ok.is_ok());
    }

    #[test]
    fn test_result_type_err() {
        let err: Result<i32> = Err(DomainError::Internal("test".to_string()));
        assert!(err.is_err());
    }
}
