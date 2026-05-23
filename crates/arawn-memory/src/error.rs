use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("entity not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),
}
