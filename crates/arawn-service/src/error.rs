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
            ServiceError::Engine(e) => Some(serde_json::json!({
                "kind": engine_error_kind(e),
            })),
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
