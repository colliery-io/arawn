use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    #[error("engine error: {0}")]
    Engine(String),

    #[error("storage error: {0}")]
    Storage(String),

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
            ServiceError::Internal(_) => "internal_error",
        }
    }
}
