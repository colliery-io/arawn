use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    #[error("engine error: {0}")]
    Engine(#[from] arawn_engine::EngineError),

    #[error("storage error: {0}")]
    Storage(#[from] arawn_storage::StorageError),

    #[error("memory error: {0}")]
    Memory(#[from] arawn_memory::MemoryError),

    #[error("internal error: {0}")]
    Internal(String),
}

impl ServiceError {
    /// Return a stable error code string for RPC responses.
    pub fn error_code(&self) -> &'static str {
        match self {
            ServiceError::NotFound(_) => "not_found",
            ServiceError::InvalidOperation(_) => "invalid_operation",
            ServiceError::Engine(_) => "engine_error",
            ServiceError::Storage(_) => "storage_error",
            ServiceError::Memory(_) => "memory_error",
            ServiceError::Internal(_) => "internal_error",
        }
    }

    /// Structured detail suitable for RPC responses. Typed sub-sources carry
    /// a `kind` tag identifying the inner variant so clients can do
    /// finer-grained dispatch without parsing the free-form message. Returns
    /// `None` for variants whose only payload is already the message.
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            ServiceError::Engine(e) => {
                let mut detail = serde_json::json!({ "kind": engine_error_kind(e) });
                // Don't flatten an LLM failure to a bare `{"kind":"llm"}` —
                // surface the inner classification (auth / model_not_found /
                // rate_limited / …) so a client can react specifically (prompt
                // for a key, suggest a model, back off) instead of showing a
                // generic "LLM error". Carry the server-suggested retry delay
                // when the provider sent one.
                if let arawn_engine::EngineError::Llm(llm) = e {
                    detail["llm_kind"] = serde_json::json!(llm.kind());
                    if let Some(d) = llm.retry_after() {
                        detail["retry_after_secs"] = serde_json::json!(d.as_secs());
                    }
                }
                Some(detail)
            }
            ServiceError::Storage(e) => Some(serde_json::json!({
                "kind": storage_error_kind(e),
            })),
            ServiceError::Memory(e) => Some(serde_json::json!({
                "kind": memory_error_kind(e),
            })),
            _ => None,
        }
    }
}

fn engine_error_kind(e: &arawn_engine::EngineError) -> &'static str {
    match e {
        arawn_engine::EngineError::Tool(_) => "tool",
        arawn_engine::EngineError::ToolNotFound(_) => "tool_not_found",
        arawn_engine::EngineError::Llm(_) => "llm",
        arawn_engine::EngineError::MaxIterations { .. } => "max_iterations",
        arawn_engine::EngineError::Other(_) => "other",
    }
}

fn storage_error_kind(e: &arawn_storage::StorageError) -> &'static str {
    match e {
        arawn_storage::StorageError::Database(_) => "database",
        arawn_storage::StorageError::Migration(_) => "migration",
        arawn_storage::StorageError::Io(_) => "io",
        arawn_storage::StorageError::Json(_) => "json",
        arawn_storage::StorageError::NotFound(_) => "not_found",
        arawn_storage::StorageError::InvalidOperation(_) => "invalid_operation",
    }
}

fn memory_error_kind(e: &arawn_memory::MemoryError) -> &'static str {
    match e {
        arawn_memory::MemoryError::Storage(_) => "storage",
        arawn_memory::MemoryError::NotFound(_) => "not_found",
        arawn_memory::MemoryError::Validation(_) => "validation",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_auth_error_surfaces_inner_kind_not_just_llm() {
        let err = ServiceError::Engine(arawn_engine::EngineError::Llm(arawn_llm::LlmError::Auth(
            "HTTP 401: bad key".into(),
        )));
        assert_eq!(err.error_code(), "engine_error");
        let details = err.details().expect("engine errors carry details");
        assert_eq!(details["kind"], "llm");
        // The whole point: a client can tell this was an auth failure.
        assert_eq!(details["llm_kind"], "auth");
    }

    #[test]
    fn llm_model_not_found_is_distinguishable() {
        let err = ServiceError::Engine(arawn_engine::EngineError::Llm(
            arawn_llm::LlmError::ModelNotFound("HTTP 404: no such model".into()),
        ));
        let details = err.details().unwrap();
        assert_eq!(details["llm_kind"], "model_not_found");
    }

    #[test]
    fn llm_rate_limited_carries_retry_after() {
        let err = ServiceError::Engine(arawn_engine::EngineError::Llm(
            arawn_llm::LlmError::RateLimited {
                message: "HTTP 429".into(),
                retry_after: Some(std::time::Duration::from_secs(7)),
            },
        ));
        let details = err.details().unwrap();
        assert_eq!(details["llm_kind"], "rate_limited");
        assert_eq!(details["retry_after_secs"], 7);
    }

    #[test]
    fn non_llm_engine_error_has_no_llm_kind() {
        let err = ServiceError::Engine(arawn_engine::EngineError::ToolNotFound("x".into()));
        let details = err.details().unwrap();
        assert_eq!(details["kind"], "tool_not_found");
        assert!(details.get("llm_kind").is_none());
    }
}
